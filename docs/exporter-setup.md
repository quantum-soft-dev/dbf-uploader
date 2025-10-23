# Data Exporter (dbf-uploader) Setup Guide

Инструкция по настройке и запуску сервиса `data_exporter` (DBF → CSV/gzip) в локальном окружении, интегрированном с `data-forge-middleware` и Keycloak.

## 1. Предварительные условия

- ✅ Запущен `data-forge-middleware` (`docker compose up -d` в соответствующем репозитории).
- ✅ Настроен Keycloak и создан `clientSecret` для сайта (см. `docs/keycloak-setup.md` в middleware).
- ✅ Установлены Rust toolchain (`rustup`, `cargo`), `PowerShell` и права на запуск консольных утилит.
- ✅ Склонирован репозиторий `dbf-uploader`.

## 2. Структура локальных директорий

```text
dbf-uploader/
 ├─ config/
 │   └─ config.toml        # Файл конфигурации v2.0
 ├─ data/
 │   └─ dbf/               # Исходные DBF-файлы (будут обработаны)
 └─ logs/
     └─ error.log          # Локальный fallback-лог
```

Создайте каталоги (если отсутствуют):

```powershell
New-Item -ItemType Directory config,data\dbf,logs -Force
```

## 3. Конфигурация `config/config.toml`

Пример рабочей конфигурации для локальной разработки:

```toml
version = "2.0"

[auth]
domain = "store01.example.com"
client_secret = "cba2c32a-f332-45d3-9956-b7f6c6eda423"

[api]
base_url = "http://localhost:8080"
auth_endpoint = "/api/v1/auth/token"
batch_start = "/api/v1/batch/start"
batch_upload = "/api/v1/batch/{batchId}/upload"
batch_complete = "/api/v1/batch/{batchId}/complete"
batch_fail = "/api/v1/batch/{batchId}/fail"
batch_cancel = "/api/v1/batch/{batchId}/cancel"
error_log = "/api/v1/error"

[source]
directory = "C:\\Users\\white\\DBF\\dbf-uploader\\data\\dbf"

[schedule]
cron = "*/5 * * * *"

[encoding]
fallback = "CP866"

[batch]
max_files_per_batch = 100
retry_locked_files = true
batch_timeout_secs = 3600
max_retries = 3
http_timeout_secs = 300
locked_file_retry_delay_secs = 5

[logging]
error_log_path = "C:\\Users\\white\\DBF\\dbf-uploader\\logs\\error.log"
```

> Значения `domain` и `client_secret` должны соответствовать сайту, созданному в middleware (см. предыдущий документ).

## 4. Переопределение пути к конфигу

По умолчанию на Windows CLI ищет файл по пути `C:\Program Files\data-exporter\config.toml`. Для разработки можно указать переменную окружения:

```powershell
$env:DATA_EXPORTER_CONFIG = "C:\Users\white\DBF\dbf-uploader\config\config.toml"
```

Чтобы сделать переопределение постоянным, выполните:

```powershell
setx DATA_EXPORTER_CONFIG "C:\Users\white\DBF\dbf-uploader\config\config.toml"
```

## 5. Сборка и тесты

```powershell
# Все юнит- и интеграционные тесты
cargo test

# Сборка релизного бинарника
cargo build --release
```

Результат: `target\release\data_exporter.exe`.

## 6. Быстрая проверка подключения

```powershell
# Exchange Basic-cred (domain:clientSecret) for JWT
$pair = "store01.example.com:cba2c32a-f332-45d3-9956-b7f6c6eda423"
$auth = [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes($pair))
Invoke-RestMethod -Uri "http://localhost:8080/api/v1/auth/token" -Method Post -Headers @{ Authorization = "Basic $auth" }
```

Если в ответ пришёл JWT-токен, конфигурация корректна и экспортер сможет общаться с middleware.

## 7. Запуск CLI

```powershell
# Интерактивный установщик (создаст config.toml и подготовит сервис)
target\release\data_exporter.exe install

# Миграция с v1.0 (заготовка, см. MIGRATION_GUIDE)
target\release\data_exporter.exe migrate

# Удаление сервисной установки
target\release\data_exporter.exe uninstall

# Одноразовый запуск batch-загрузки по текущему config.toml
target\release\data_exporter.exe run
```

Для разработки можно работать напрямую с `config/config.toml`, не устанавливая сервис.

## 8. Обработка файлов

1. Поместите *.dbf файлы в `data\dbf`.
2. Запустите ручной сценарий (пока scheduler закомментирован):
   ```powershell
   cargo run --release -- --help   # изучить команды
   ```
3. При реализации Phase 3 (BatchManager) загрузки будут выполняться через CLI-команды.

## 9. Локальный fallback-лог

Если отправка ошибок в middleware не удалась, записи попадают в `logs\error.log`. Проверьте его, если видите предупреждения по batch-операциям на стороне сервера.

## 10. Типичные проблемы

| Проблема | Причина | Решение |
|----------|---------|---------|
| 401 Unauthorized при запросе токена | Неверный `domain`/`client_secret` или сайт деактивирован | Пересоздайте сайт в middleware, обновите config.toml |
| Ошибки TLS/host mismatch | Используете HTTPS без настроенного сертификата | Для dev оставьте `http://localhost:8080` или настройте прокси с TLS |
| Приложение не находит конфиг | Не установлена переменная `DATA_EXPORTER_CONFIG` | Установите переменную или создайте файл по пути по умолчанию |
| Middleware не принимает запросы | Сервис `dfm-backend` не стартовал или не прошёл healthcheck | Проверьте `docker compose ps` и логи `dfm-backend` |

## 11. Переход в production

- Храните конфиг и секреты в защищённом vault (например, HashiCorp Vault, Azure Key Vault).
- Запускайте `data_exporter` как Windows Service (реализация в процессе) с выделенным техническим пользователем.
- Переключите `base_url` на HTTPS (домен, защищённый TLS).
- Подключите мониторинг логов и планировщик задач согласно корпоративному регламенту.

---

Следуя этим шагам, можно локально настроить и проверить взаимодействие экспортёра данных с middleware и Keycloak. Для адаптации под продакшен используйте соответствующие регламенты по безопасности и операционной эксплуатации.

