# PRD: Data Exporter Service - Сервисная часть

**Продукт**: Data Exporter Windows Service
**Версия**: 1.0
**Дата**: 2026-01-24

---

## 1. Обзор продукта

Data Exporter Service - Windows-служба для автоматического экспорта DBF файлов в CSV с загрузкой на удалённый сервер. Работает в фоновом режиме по cron-расписанию.

**Границы PRD**:
- Scheduler и планирование задач
- Pipeline обработки файлов (scan -> convert -> compress -> upload)
- VSS для заблокированных файлов
- Обработка ошибок и retry логика
- Надёжность логирования при падениях

**Исключено из PRD**:
- GUI Configurator
- CLI install/uninstall
- Device Flow авторизация (только TokenManager)

---

## 2. Функциональные требования

### 2.1 Windows Service Lifecycle

| ID | Требование |
|----|------------|
| FR-SVC-001 | Регистрация в SCM под именем `data-exporter` |
| FR-SVC-002 | Поддержка команд Start, Stop, Interrogate |
| FR-SVC-003 | Graceful shutdown при Stop сигнале |
| FR-SVC-004 | Логирование в `C:\Program Files\data-exporter\logs\` с ежедневной ротацией |
| FR-SVC-005 | Загрузка config.toml с retry при недоступности source_dir (1,2,4,8,16 мин -> ежечасно) |

### 2.2 Scheduler (Cron)

| ID | Требование |
|----|------------|
| FR-SCH-001 | 5-field cron формат (min hour day month weekday) |
| FR-SCH-002 | Автоконверсия в 6-field (добавление `0` для секунд) |
| FR-SCH-003 | Batch lock - предотвращение одновременного выполнения |
| FR-SCH-004 | Hot-reload конфигурации при каждом batch |

### 2.3 Directory Scanning

| ID | Требование |
|----|------------|
| FR-SCN-001 | Рекурсивный обход source_dir |
| FR-SCN-002 | Поиск файлов `.dbf` (case-insensitive) |
| FR-SCN-003 | Include/exclude glob patterns |
| FR-SCN-004 | Автоопределение кодировки из DBF header |

### 2.4 File Processing Pipeline

| ID | Требование |
|----|------------|
| FR-CNV-001 | Shared read access (FILE_SHARE_READ) для открытия файлов |
| FR-CNV-002 | Кодировки: CP866, Windows-1251, Windows-1255, ISO-8859-8, UTF-8 |
| FR-CNV-003 | Конвертация всех данных в UTF-8 CSV |
| FR-CNV-004 | Пропуск повреждённых записей с предупреждением |
| FR-MEM-001 | In-memory до 10 MB, иначе temp file |
| FR-MEM-002 | Автоудаление temp файлов через Drop trait |
| FR-CMP-001 | GZIP сжатие с default compression |
| FR-UPL-001 | Multipart upload с JWT Bearer auth |
| FR-UPL-002 | Retry 3x с exponential backoff (1s, 2s, 4s) для 5xx/network errors |
| FR-UPL-003 | Timeout 300 сек для upload |

### 2.5 VSS для заблокированных файлов

| ID | Требование |
|----|------------|
| FR-VSS-001 | Детекция locked файлов (OS error 32, 33) |
| FR-VSS-002 | Отложенная обработка через rawcopy |
| FR-VSS-003 | Temp директория `%TEMP%\dbf_vss_{batch_id}\` |
| FR-VSS-004 | Автоочистка VSS копий после обработки |

### 2.6 Batch Lifecycle

| ID | Требование |
|----|------------|
| FR-BCH-001 | `POST /batches/start` -> получить batch_id |
| FR-BCH-002 | `POST /batches/{id}/complete` при успехе |
| FR-BCH-003 | `POST /batches/{id}/complete-with-warnings` при warnings |
| FR-BCH-004 | `POST /batches/{id}/fail` при critical errors |

---

## 3. Классификация ошибок

### Critical (abort batch):
- `DirectoryInaccessible`
- `AuthenticationError`
- `ConfigurationError`

### Warning (continue batch):
- `FileReadError`, `ConversionError`, `EncodingError`
- `CompressionError`, `UploadError`, `NetworkError`
- `VssError`, `DiskFullError`

---

## 3.1 Надёжность логирования при падениях (КРИТИЧНО)

Сервис **ДОЛЖЕН** оставить запись в логе при любом неожиданном завершении.

### Текущее состояние (анализ кода):

| Сценарий | Текущее поведение | Статус |
|----------|-------------------|--------|
| Ошибка создания log directory | Пишет в `C:\Windows\Temp\data_exporter_log_error.txt`, exit(1) | Частично |
| Ошибка создания tokio runtime | Логирует через tracing, возвращает ошибку | OK |
| Ошибка загрузки конфига | Логирует через tracing, retry или возврат ошибки | OK |
| Ошибка регистрации control handler | Логирует через tracing | OK |
| Ошибка создания scheduler | Логирует `error!`, но не устанавливает статус Stopped | Проблема |
| Ошибка запуска scheduler | Логирует `error!`, но не устанавливает статус Stopped | Проблема |
| Panic в async коде | НЕ обрабатывается - сервис падает без лога | КРИТИЧНО |
| Out of memory | НЕ обрабатывается | КРИТИЧНО |

### Требования (FR-LOG):

| ID | Требование | Приоритет |
|----|------------|-----------|
| FR-LOG-001 | При ЛЮБОЙ ошибке инициализации сервис ДОЛЖЕН записать причину в лог | P0 |
| FR-LOG-002 | При panic сервис ДОЛЖЕН записать stack trace в лог через `std::panic::set_hook` | P0 |
| FR-LOG-003 | Fallback логирование в `C:\Windows\Temp\data_exporter_*.txt` при недоступности основного лога | P0 |
| FR-LOG-004 | При ошибке scheduler сервис ДОЛЖЕН установить статус Stopped и записать ошибку | P1 |
| FR-LOG-005 | Логирование должно быть синхронным (flush) при критических ошибках | P1 |

### Рекомендуемые изменения:

```rust
// 1. Добавить panic hook в начале service_main
std::panic::set_hook(Box::new(|panic_info| {
    let msg = format!("PANIC: {}\n{:?}", panic_info, std::backtrace::Backtrace::capture());
    // Fallback логирование
    let _ = std::fs::write(
        format!(r"C:\Windows\Temp\data_exporter_panic_{}.txt",
                std::process::id()),
        &msg
    );
    // Попытка записать в основной лог
    error!("{}", msg);
}));

// 2. Обернуть весь async блок в catch_unwind
let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
    runtime.block_on(async { ... })
}));

// 3. Гарантировать установку статуса Stopped при любом выходе
// (использовать Drop guard или defer pattern)
```

### Fallback логирование:

При недоступности основного лога (`C:\Program Files\data-exporter\logs\`):
1. Записать в `C:\Windows\Temp\data_exporter_error_{pid}_{timestamp}.txt`
2. Записать в Windows Event Log (опционально)
3. Записать в stderr (для отладки)

### Проверка (тесты):

1. **Тест panic**: вызвать panic в scheduler и проверить наличие лога
2. **Тест недоступного лога**: заблокировать log директорию и проверить fallback
3. **Тест ошибки scheduler**: сломать crontab и проверить корректный статус Stopped

---

## 3.2 Standalone Error Logging API (не связанное с batch)

Для ошибок, не связанных с batch-обработкой (проблемы с директорией, ошибки формата crontab и т.д.), используется отдельный API endpoint.

### Log Standalone Error

**Endpoint:** `POST /api/v1/device/errors`

**Использование:** Для логирования ошибок инициализации и конфигурации:
- Недоступность source_dir
- Ошибка парсинга crontab
- Ошибка подключения к API
- Panic в сервисе
- Ошибки токенизации

**Request:**
```http
POST /api/v1/device/errors HTTP/1.1
Authorization: Bearer <jwt-token>
Content-Type: application/json

{
  "type": "CONFIGURATION_ERROR",
  "message": "Invalid crontab format: expected 5 fields, got 3",
  "metadata": {
    "crontab": "* * *",
    "configPath": "C:\\Program Files\\data-exporter\\config.toml",
    "clientVersion": "1.0.0"
  }
}
```

**Response:**
```json
{
  "id": "error-uuid-...",
  "batchId": null,
  "severity": "ERROR",
  "message": "Invalid crontab format: expected 5 fields, got 3",
  "source": "DEVICE",
  "metadata": { ... },
  "occurredAt": "2025-11-06T12:32:00Z"
}
```

### Error Types для Standalone Errors:

| Type | Описание | Severity |
|------|----------|----------|
| `CONFIGURATION_ERROR` | Ошибки конфигурации (TOML, crontab) | ERROR |
| `DIRECTORY_INACCESSIBLE` | Source directory недоступна | CRITICAL |
| `CONNECTION_ERROR` | Ошибка подключения к API | ERROR |
| `AUTHENTICATION_ERROR` | Ошибка аутентификации | CRITICAL |
| `SERVICE_PANIC` | Panic в сервисе | CRITICAL |
| `INITIALIZATION_ERROR` | Ошибки инициализации (runtime, scheduler) | ERROR |

### Severity Levels:

| Severity | Описание |
|----------|----------|
| `CRITICAL` | Сервис не может продолжать работу |
| `ERROR` | Серьёзная ошибка, но сервис может работать |
| `WARNING` | Предупреждение, не влияет на работу |
| `INFO` | Информационное сообщение |

### Get Error Details

**Endpoint:** `GET /api/v1/device/errors/{errorId}`

### Интеграция в сервис (требования):

| ID | Требование |
|----|------------|
| FR-ERR-001 | При ошибках инициализации отправлять standalone error на сервер |
| FR-ERR-002 | При недоступности API - fallback на локальное логирование |
| FR-ERR-003 | Включать metadata с версией клиента, путями, деталями ошибки |
| FR-ERR-004 | Retry при отправке с exponential backoff (3 попытки) |

### Примеры использования:

```rust
// Ошибка crontab
error_reporter.send_standalone_error(
    "CONFIGURATION_ERROR",
    "Invalid crontab format",
    json!({
        "crontab": config.scheduler.crontab,
        "configPath": config_path.display().to_string(),
    }),
    Severity::Error,
).await;

// Panic
error_reporter.send_standalone_error(
    "SERVICE_PANIC",
    &format!("PANIC: {}", panic_info),
    json!({
        "backtrace": backtrace.to_string(),
        "threadId": std::thread::current().id(),
    }),
    Severity::Critical,
).await;

// Недоступная директория
error_reporter.send_standalone_error(
    "DIRECTORY_INACCESSIBLE",
    &format!("Source directory not accessible: {}", source_dir.display()),
    json!({
        "sourceDir": source_dir.display().to_string(),
        "osError": os_error.to_string(),
    }),
    Severity::Critical,
).await;
```

---

## 4. Pipeline обработки

```
+-------------------------------------------------------------+
|                    BATCH PROCESSING                         |
+-------------------------------------------------------------+
|                                                             |
|  1. START BATCH ON SERVER                                   |
|     +-- POST /batches/start -> batch_id                     |
|                                                             |
|  2. SCAN DIRECTORY                                          |
|     +-- Recursive walk -> filter -> detect encoding         |
|                                                             |
|  3. FOR EACH DBF FILE:                                      |
|     +----------+    +----------+    +----------+            |
|     | CONVERT  |--->| COMPRESS |--->|  UPLOAD  |            |
|     | DBF->CSV |    | CSV->GZ  |    | to API   |            |
|     +----------+    +----------+    +----------+            |
|         |               |               |                   |
|     <=10MB: RAM     GZIP default    Retry 3x                |
|     >10MB: File                     exp. backoff            |
|                                                             |
|     ON LOCKED: defer to locked_files list                   |
|     ON ERROR:  report, mark failed, continue                |
|                                                             |
|  4. RETRY LOCKED FILES (VSS)                                |
|     +-- rawcopy -> process -> cleanup                       |
|                                                             |
|  5. COMPLETE BATCH                                          |
|     +-- Critical errors -> FAIL                             |
|     +-- Warnings only   -> COMPLETE_WITH_WARNINGS           |
|     +-- No errors       -> COMPLETE                         |
|                                                             |
+-------------------------------------------------------------+
```

---

## 5. Критические файлы

| Файл | Назначение |
|------|------------|
| `service/src/processor/mod.rs` | Оркестрация batch (run_batch) |
| `service/src/service/scheduler.rs` | Cron scheduler с batch lock |
| `service/src/service/windows_service.rs` | Windows Service lifecycle |
| `service/src/processor/converter.rs` | DBF->CSV конвертация |
| `service/src/processor/scanner.rs` | Сканирование директории |
| `service/src/processor/compressor.rs` | GZIP сжатие |
| `service/src/processor/uploader.rs` | Upload с retry |
| `service/src/vss/mod.rs` | VSS copy для locked файлов |
| `common/src/error/mod.rs` | Классификация ошибок |
| `common/src/auth/mod.rs` | TokenManager |

---

## 6. Зависимости (Cargo)

| Crate | Назначение |
|-------|------------|
| `tokio` | Async runtime |
| `windows-service` | Windows Service API |
| `tokio-cron-scheduler` | Cron scheduling |
| `reqwest` | HTTP client |
| `dbase` | DBF parsing |
| `csv` | CSV writing |
| `flate2` | GZIP compression |
| `rawcopy-rs` | VSS file copy |
| `globset` | Glob pattern matching |
| `tracing` | Logging |

---

## 7. API Endpoints

### Batch-related:

| Endpoint | Method | Назначение |
|----------|--------|------------|
| `/api/v1/device/auth/token` | POST | JWT token (Basic Auth) |
| `/api/v1/device/batches/start` | POST | Создание batch |
| `/api/v1/device/batches/{id}/complete` | POST | Завершение OK |
| `/api/v1/device/batches/{id}/complete-with-warnings` | POST | Завершение с warnings |
| `/api/v1/device/batches/{id}/fail` | POST | Завершение с ошибкой |
| `/api/v1/device/files/batches/{id}/upload` | POST | Upload файла |

### Error Logging:

| Endpoint | Method | Назначение |
|----------|--------|------------|
| `/api/v1/device/errors` | POST | Standalone error (не связанный с batch) |
| `/api/v1/device/errors/{errorId}` | GET | Получить детали ошибки |
| `/api/v1/errors/report` | POST | Batch-related error report |

---

## 8. Конфигурация (config.toml)

```toml
[scheduler]
crontab = "0 8,12,16,18 * * *"

[src]
source_dir = "C:\\data"
include_patterns = ["*.dbf"]
exclude_patterns = ["temp_*.dbf"]

[credential]
# Traditional или Device Flow auth

[api]
base_url = "https://api.example.com"
https_only = true

[encoding]
dbf_encoding = "CP866"  # Fallback
```

---

## 9. Верификация

Для тестирования изменений:

1. **Unit tests**: `cargo test -p service`
2. **Ручное тестирование**:
   - Создать тестовую директорию с DBF файлами
   - Запустить сервис через `cargo run -p service`
   - Проверить логи в `logs/service.log`
   - Проверить batch status на сервере
3. **Тест locked файлов**: открыть DBF в другом приложении и проверить VSS retry
