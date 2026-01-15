# Client-Server Integration Guide

**Project**: DBF Uploader - Windows Service для экспорта DBF файлов в CSV и загрузки на сервер
**Version**: 1.0.0
**Last Updated**: 2026-01-12
**Status**: Implementation Complete

---

## Содержание

1. [Обзор архитектуры](#обзор-архитектуры)
2. [API Endpoints](#api-endpoints)
3. [Жизненный цикл батча](#жизненный-цикл-батча)
4. [Текущая реализация](#текущая-реализация)
5. [Недостающие компоненты](#недостающие-компоненты)
6. [План интеграции](#план-интеграции)
7. [Тестирование интеграции](#тестирование-интеграции)
8. [Мониторинг и отладка](#мониторинг-и-отладка)

---

## Обзор архитектуры

### Компоненты системы

```
┌─────────────────────────────────────────────────────────────┐
│                    Windows Service (Client)                 │
│                                                              │
│  ┌────────────┐  ┌──────────────┐  ┌────────────────────┐  │
│  │ Scheduler  │──│ Batch        │──│ File Processor    │  │
│  │ (Cron)     │  │ Processor    │  │ (DBF→CSV→GZ)      │  │
│  └────────────┘  └──────────────┘  └────────────────────┘  │
│        │               │                     │               │
│        └───────────────┼─────────────────────┘               │
│                        │                                     │
│        ┌───────────────┼────────────────────┐               │
│        │               │                    │               │
│        ▼               ▼                    ▼               │
│  ┌──────────┐  ┌──────────────┐  ┌────────────────┐       │
│  │  Auth    │  │  Batch       │  │  Error         │       │
│  │  Client  │  │  Client      │  │  Reporter      │       │
│  └──────────┘  └──────────────┘  └────────────────┘       │
│        │               │                    │               │
└────────┼───────────────┼────────────────────┼───────────────┘
         │               │                    │
         │ HTTPS         │ HTTPS              │ HTTPS
         ▼               ▼                    ▼
┌────────────────────────────────────────────────────────────┐
│                   Cloud API Server                         │
│                                                            │
│  ┌────────────┐  ┌──────────────┐  ┌────────────────┐    │
│  │  /api/     │  │  /api/v1/    │  │  /api/         │    │
│  │  auth/     │  │  device/     │  │  files/        │    │
│  │  token     │  │  batches/*   │  │  upload        │    │
│  └────────────┘  └──────────────┘  └────────────────┘    │
│                                                            │
│  ┌───────────────────────────────────────────────────┐    │
│  │  /api/errors/report                               │    │
│  └───────────────────────────────────────────────────┘    │
└────────────────────────────────────────────────────────────┘
```

### Поток данных

1. **Аутентификация**: Client → `/api/auth/token` → Получение JWT
2. **Старт батча**: Client → `/api/v1/device/batches/start` → Batch ID
3. **Обработка файлов**: DBF → CSV → GZIP → `/api/files/upload`
4. **Репорт ошибок**: `/api/errors/report` (при сбоях)
5. **Завершение батча**: Client → `/api/v1/device/batches/complete` (или fail/complete-with-warnings)

---

## API Endpoints

### 1. Authentication API

**Endpoint**: `POST /api/auth/token`
**Status**: ✅ Реализовано в `src/auth/mod.rs`

#### Request
```http
POST /api/auth/token HTTP/1.1
Authorization: Basic <base64(username:password)>
Content-Type: application/json
```

#### Response (200 OK)
```json
{
  "token": "eyJhbGci...",
  "expires_in": 86400
}
```

#### Текущая реализация
- **Файл**: `src/auth/mod.rs`
- **Struct**: `TokenManager`
- **Методы**:
  - `new()` - создание с credentials из config
  - `get_valid_token()` - получение токена с авто-обновлением
  - `request_token()` - запрос нового токена
  - `is_token_expired()` - проверка истечения срока

#### Особенности
- ✅ Автоматическое обновление токена при истечении
- ✅ Basic Auth с base64 encoding
- ✅ Хранение токена только в памяти
- ✅ Проверка за 5 минут до истечения

---

### 2. Batch Management API

**Base Path**: `/api/v1/device/batches`
**Status**: ✅ Реализовано в `src/processor/batch_client.rs`

#### 2.1 Start Batch

**Endpoint**: `POST /api/v1/device/batches/start`

**Request**
```http
POST /api/v1/device/batches/start HTTP/1.1
Authorization: Bearer <jwt_token>
Content-Type: application/json
```

**Response (200 OK)**
```json
{
  "id": "8e493618-d745-4c29-86f5-db54d39bd71d"
}
```

**Реализация**: `BatchClient::start_batch()`

#### 2.2 Complete Batch

**Endpoint**: `POST /api/v1/device/batches/{batchId}/complete`

**Request**
```http
POST /api/v1/device/batches/{batchId}/complete HTTP/1.1
Authorization: Bearer <jwt_token>
```

**Response (200 OK)**
```json
{
  "status": "completed"
}
```

**Реализация**: `BatchClient::complete_batch()`

**Когда использовать**: Все файлы успешно обработаны без ошибок

#### 2.3 Complete Batch with Warnings

**Endpoint**: `POST /api/v1/device/batches/{batchId}/complete-with-warnings`

**Request**
```http
POST /api/v1/device/batches/{batchId}/complete-with-warnings HTTP/1.1
Authorization: Bearer <jwt_token>
```

**Response (200 OK)**
```json
{
  "status": "completed_with_warnings"
}
```

**Реализация**: `BatchClient::complete_with_warnings()`

**Когда использовать**:
- Некоторые файлы упали с некритичными ошибками (corrupted files, encoding errors)
- Хотя бы один файл обработан успешно
- Ни одной критичной ошибки (no directory access, auth failure)

#### 2.4 Fail Batch

**Endpoint**: `POST /api/v1/device/batches/{batchId}/fail`

**Request**
```http
POST /api/v1/device/batches/{batchId}/fail HTTP/1.1
Authorization: Bearer <jwt_token>
```

**Response (200 OK)**
```json
{
  "status": "failed"
}
```

**Реализация**: `BatchClient::fail_batch()`

**Когда использовать**:
- Критичная ошибка (directory inaccessible, auth failure, config error)
- Ни одного файла не обработано (processed_count = 0)

#### 2.5 Cancel Batch

**Endpoint**: `POST /api/v1/device/batches/{batchId}/cancel`

**Request**
```http
POST /api/v1/device/batches/{batchId}/cancel HTTP/1.1
Authorization: Bearer <jwt_token>
```

**Response (200 OK)**
```json
{
  "status": "cancelled"
}
```

**Реализация**: `BatchClient::cancel_batch()`

**Когда использовать**: Service stop signal получен во время обработки батча (пока не реализовано)

---

### 3. File Upload API

**Endpoint**: `POST /api/files/upload`
**Status**: ✅ Реализовано в `src/processor/uploader.rs`

#### Request
```http
POST /api/files/upload HTTP/1.1
Authorization: Bearer <jwt_token>
Content-Type: multipart/form-data; boundary=----WebKit...
Content-Encoding: gzip

------WebKit...
Content-Disposition: form-data; name="file"; filename="subdir_data.csv.gz"
Content-Type: application/gzip

<binary gzip data>
------WebKit...--
```

#### Response (200 OK)
```json
{
  "status": "success",
  "message": "File uploaded successfully",
  "file_id": "abc-123-xyz"
}
```

#### Текущая реализация
- **Файл**: `src/processor/uploader.rs`
- **Функция**: `upload_file()`
- **Retry logic**: 3 попытки с exponential backoff (1s, 2s, 4s)
- **Обработка ошибок**:
  - 401: Обновление токена и повтор
  - 4xx: Логирование и skip
  - 5xx: Retry с backoff
  - Network errors: Retry

#### Особенности
- ✅ Multipart/form-data upload
- ✅ GZIP compression
- ✅ Filename transformation (path separators → underscores)
- ✅ Retry с exponential backoff
- ✅ Автоматическое удаление локального CSV после upload

---

### 4. Error Reporting API

**Endpoint**: `POST /api/errors/report`
**Status**: ✅ Реализовано в `src/error/reporter.rs`

#### Request
```http
POST /api/errors/report HTTP/1.1
Authorization: Bearer <jwt_token>
Content-Type: application/json

{
  "filename": "subdir\\data.dbf",
  "error_type": "FileReadError",
  "message": "Failed to read DBF file: Permission denied (OS Error 5)",
  "error_details": "FileReadError: Failed to read DBF file\nCaused by: IO Error: Permission denied",
  "timestamp": "2026-01-12T14:30:00Z",
  "client_version": "1.0.0"
}
```

#### Response (200 OK)
```json
{
  "status": "received",
  "error_id": "error-abc-123"
}
```

#### Текущая реализация
- **Файл**: `src/error/reporter.rs`
- **Struct**: `ErrorReporter`
- **Метод**: `report_error()`
- **Fallback**: Local error.log при сбое отправки на сервер

#### Поддерживаемые типы ошибок
```rust
enum ProcessingError {
    FileReadError,          // Не удалось прочитать DBF
    EncodingError,          // Ошибка кодировки
    ConversionError,        // Ошибка конвертации DBF → CSV
    CompressionError,       // Ошибка сжатия GZIP
    UploadError,            // Ошибка загрузки файла
    NetworkError,           // Сетевая ошибка
    DiskFullError,          // Нет места на диске
    DirectoryInaccessible,  // Директория недоступна
    AuthenticationError,    // Ошибка аутентификации
    ConfigurationError,     // Ошибка конфигурации
    VssError,              // Ошибка VSS (Volume Shadow Copy)
}
```

#### Особенности
- ✅ Полная цепочка ошибок в `error_details` (source chain)
- ✅ Fire-and-forget (нет retry)
- ✅ Local fallback при network errors
- ✅ ISO 8601 timestamp format
- ✅ Client version tracking

---

## Жизненный цикл батча

### Состояния батча

```
                    ┌─────────────┐
                    │   START     │
                    │  (Initial)  │
                    └──────┬──────┘
                           │
                           ▼
                    ┌─────────────┐
                    │  PROCESSING │
                    │  (Running)  │
                    └──────┬──────┘
                           │
                ┌──────────┼──────────┐
                │          │          │
                ▼          ▼          ▼
         ┌──────────┐ ┌──────────┐ ┌──────────────┐
         │ FAILED   │ │COMPLETED │ │ COMPLETED_   │
         │          │ │          │ │ WITH_WARNINGS│
         └──────────┘ └──────────┘ └──────────────┘
                │          │          │
                └──────────┴──────────┘
                           │
                           ▼
                    ┌─────────────┐
                    │     END     │
                    └─────────────┘
```

### Логика переходов

#### → FAILED
**Условия** (`src/processor/mod.rs:280`):
```rust
if critical_errors > 0 || batch.processed_count == 0 {
    batch_client.fail_batch(&batch_id, &token, &config).await?;
}
```
- **Критичные ошибки**: DirectoryInaccessible, AuthenticationError, ConfigurationError
- **Пустой батч**: Ни одного файла не обработано

#### → COMPLETED_WITH_WARNINGS
**Условия** (`src/processor/mod.rs:295`):
```rust
else if warnings > 0 {
    batch_client.complete_with_warnings(&batch_id, &token, &config).await?;
}
```
- **Некритичные ошибки**: FileReadError, ConversionError, EncodingError, и т.д.
- **Хотя бы один файл обработан**: processed_count > 0

#### → COMPLETED
**Условия** (`src/processor/mod.rs:311`):
```rust
else {
    batch_client.complete_batch(&batch_id, &token, &config).await?;
}
```
- **Нет ошибок**: warnings = 0, critical_errors = 0
- **Все файлы успешно обработаны**

### Пример сценария

#### Сценарий 1: Успешный батч
```
Файлов найдено: 10
Обработано: 10
Ошибок: 0
→ COMPLETED
```

#### Сценарий 2: Батч с warnings
```
Файлов найдено: 10
Обработано: 8
Испорченные файлы: 2 (corrupted DBF)
Warnings: 2
Critical errors: 0
→ COMPLETED_WITH_WARNINGS
```

#### Сценарий 3: Критичная ошибка
```
Файлов найдено: 10
Обработано: 0
Ошибка: Network share unavailable
Critical errors: 1
→ FAILED
```

#### Сценарий 4: Все файлы упали
```
Файлов найдено: 5
Обработано: 5
Все файлы упали (corrupted): 5
Warnings: 5
Critical errors: 0
processed_count: 5 (попытки обработки)
completed_count: 0 (успешные)
→ COMPLETED_WITH_WARNINGS (т.к. processed_count > 0)
```

---

## Текущая реализация

### ✅ Реализованные компоненты

| Компонент | Файл | Статус | Функциональность |
|-----------|------|--------|------------------|
| **Auth Client** | `src/auth/mod.rs` | ✅ Complete | JWT token management с auto-renewal |
| **Batch Client** | `src/processor/batch_client.rs` | ✅ Complete | Start, Complete, Complete-with-warnings, Fail, Cancel |
| **File Uploader** | `src/processor/uploader.rs` | ✅ Complete | Multipart upload с retry logic |
| **Error Reporter** | `src/error/reporter.rs` | ✅ Complete | Error reporting с local fallback |
| **Batch Processor** | `src/processor/mod.rs` | ✅ Complete | Полный цикл обработки батча |
| **Config Manager** | `src/models/config.rs` | ✅ Complete | TOML config с validation |
| **Scheduler** | `src/service/scheduler.rs` | ✅ Complete | Cron-based scheduling |
| **Windows Service** | `src/service/windows_service.rs` | ✅ Complete | Service lifecycle management |

### 🔧 Дополнительные фичи

| Фича | Статус | Описание |
|------|--------|----------|
| **Network retry** | ✅ | Exponential backoff для сетевых запросов |
| **Config hot-reload** | ✅ | Автоматическое применение изменений config.toml |
| **Locked file handling** | ✅ | VSS retry для заблокированных файлов |
| **Error classification** | ✅ | Critical vs Warning errors |
| **Detailed error reporting** | ✅ | Full error chain в error_details |
| **Service startup retry** | ✅ | Retry при недоступности network share (1,2,4,8,16 min, then hourly) |

---

## Недостающие компоненты

### ⚠️ API endpoints на сервере (требуется реализация)

#### 1. Batch Management Endpoints

**Должны быть реализованы на сервере**:

```typescript
// TypeScript примеры (сервер)

// Start batch
POST /api/v1/device/batches/start
Authorization: Bearer <token>
Response: { id: string }

// Complete batch
POST /api/v1/device/batches/{batchId}/complete
Authorization: Bearer <token>
Response: { status: "completed" }

// Complete with warnings
POST /api/v1/device/batches/{batchId}/complete-with-warnings
Authorization: Bearer <token>
Response: { status: "completed_with_warnings" }

// Fail batch
POST /api/v1/device/batches/{batchId}/fail
Authorization: Bearer <token>
Response: { status: "failed" }

// Cancel batch
POST /api/v1/device/batches/{batchId}/cancel
Authorization: Bearer <token>
Response: { status: "cancelled" }
```

#### 2. Error Reporting Endpoint

```typescript
POST /api/errors/report
Authorization: Bearer <token>
Content-Type: application/json

Request Body:
{
  filename: string,
  error_type: string,
  message: string,
  error_details?: string,  // NEW: full error chain
  timestamp: string,       // ISO 8601
  client_version: string,
  metadata?: {
    filename: string,
    clientVersion: string,
    timestamp: string
  }
}

Response: { status: "received", error_id?: string }
```

### 📋 Серверная логика

#### Batch Management
- **Создание батча**: Генерация UUID, сохранение в БД со статусом "processing"
- **Обновление статуса**: Переход между состояниями (completed/failed/completed_with_warnings)
- **Tracking**: Связь batch → uploaded files
- **Metrics**: Подсчет успешных/failed файлов

#### Error Reporting
- **Логирование**: Сохранение error reports в БД
- **Aggregation**: Группировка по error_type, filename, client_version
- **Alerting**: Уведомления при критических ошибках
- **Analytics**: Dashboard для мониторинга

---

## План интеграции

### Этап 1: Базовая интеграция

**Приоритет**: 🔴 Критический
**Время**: 2-3 дня

#### Задачи

1. **Реализовать batch management endpoints на сервере**
   - [ ] `POST /api/v1/device/batches/start`
   - [ ] `POST /api/v1/device/batches/{id}/complete`
   - [ ] `POST /api/v1/device/batches/{id}/fail`

2. **Создать БД таблицу для батчей**
   ```sql
   CREATE TABLE batches (
     id UUID PRIMARY KEY,
     device_id UUID NOT NULL,
     status VARCHAR(50) NOT NULL,
     started_at TIMESTAMP NOT NULL,
     completed_at TIMESTAMP,
     files_processed INT DEFAULT 0,
     files_failed INT DEFAULT 0,
     created_at TIMESTAMP DEFAULT NOW(),
     updated_at TIMESTAMP DEFAULT NOW()
   );
   ```

3. **Реализовать error reporting endpoint**
   - [ ] `POST /api/errors/report`
   - [ ] БД таблица для error logs

4. **Базовое тестирование**
   - [ ] Unit tests для endpoints
   - [ ] Integration test: полный цикл батча

### Этап 2: Расширенная функциональность

**Приоритет**: 🟡 Высокий
**Время**: 2-3 дня

#### Задачи

1. **Complete with warnings endpoint**
   - [ ] `POST /api/v1/device/batches/{id}/complete-with-warnings`
   - [ ] Обновление БД схемы для warnings

2. **Cancel batch endpoint**
   - [ ] `POST /api/v1/device/batches/{id}/cancel`
   - [ ] Graceful cancellation logic

3. **Error details support**
   - [ ] Расширение error_report schema для `error_details` field
   - [ ] Full error chain parsing

4. **Metrics и monitoring**
   - [ ] Batch success/failure rates
   - [ ] Error type distribution
   - [ ] Client version tracking

### Этап 3: Оптимизация и мониторинг

**Приоритет**: 🟢 Средний
**Время**: 1-2 дня

#### Задачи

1. **Dashboard для мониторинга**
   - [ ] Batch status overview
   - [ ] Error trends visualization
   - [ ] Device health monitoring

2. **Alerting**
   - [ ] Email/Slack notifications при critical errors
   - [ ] Batch failure alerts
   - [ ] Network connectivity issues

3. **Performance optimization**
   - [ ] Index optimization для batch queries
   - [ ] Batch status caching
   - [ ] Error log archival strategy

### Этап 4: Advanced Features

**Приоритет**: 🔵 Низкий
**Время**: 2-3 дня

#### Задачи

1. **Batch analytics**
   - [ ] Processing time tracking
   - [ ] File size distribution
   - [ ] Peak usage analysis

2. **Error recovery**
   - [ ] Automatic retry suggestions
   - [ ] Error pattern detection
   - [ ] Client version compatibility warnings

3. **Audit logging**
   - [ ] Full batch lifecycle audit trail
   - [ ] Admin action logging
   - [ ] Compliance reporting

---

## Тестирование интеграции

### Unit Tests

#### Client-side (Rust)

```rust
// src/processor/batch_client.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_start_batch_success() {
        // Mock server response
        // Assert batch ID returned
    }

    #[tokio::test]
    async fn test_complete_batch_with_warnings() {
        // Test complete-with-warnings endpoint
    }

    #[tokio::test]
    async fn test_batch_failure_on_critical_error() {
        // Test fail_batch call
    }
}
```

#### Server-side (TypeScript/Node)

```typescript
describe('Batch Management API', () => {
  it('should start a new batch', async () => {
    const response = await request(app)
      .post('/api/v1/device/batches/start')
      .set('Authorization', `Bearer ${token}`)
      .expect(200);

    expect(response.body).toHaveProperty('id');
  });

  it('should complete batch with warnings', async () => {
    const batchId = await createBatch();

    const response = await request(app)
      .post(`/api/v1/device/batches/${batchId}/complete-with-warnings`)
      .set('Authorization', `Bearer ${token}`)
      .expect(200);

    expect(response.body.status).toBe('completed_with_warnings');
  });
});
```

### Integration Tests

#### End-to-End Test Scenarios

**Scenario 1: Успешный батч**
```gherkin
Given Windows service запущен
And Network share доступен
And 10 DBF файлов в source directory
When Scheduled time наступает
Then Batch стартует на сервере
And Все 10 файлов конвертируются
And Все 10 файлов загружаются
And Batch завершается со статусом COMPLETED
```

**Scenario 2: Батч с warnings**
```gherkin
Given 5 valid DBF файлов
And 3 corrupted DBF файла
When Batch обрабатывается
Then 5 файлов загружаются успешно
And 3 error reports отправляются
And Batch завершается со статусом COMPLETED_WITH_WARNINGS
```

**Scenario 3: Critical failure**
```gherkin
Given Network share недоступен
When Service пытается запустить batch
Then Service retries 5 раз (1,2,4,8,16 min)
And После retries создает batch
And Batch fail сразу с DirectoryInaccessible
And Batch статус = FAILED
```

### Mock Server для локального тестирования

```rust
// tests/mock_server.rs
use wiremock::{MockServer, Mock, ResponseTemplate};
use wiremock::matchers::{method, path};

async fn setup_mock_server() -> MockServer {
    let mock_server = MockServer::start().await;

    // Mock auth endpoint
    Mock::given(method("POST"))
        .and(path("/api/auth/token"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(json!({
                "token": "mock-jwt-token",
                "expires_in": 86400
            })))
        .mount(&mock_server)
        .await;

    // Mock batch start
    Mock::given(method("POST"))
        .and(path("/api/v1/device/batches/start"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(json!({
                "id": "test-batch-id-123"
            })))
        .mount(&mock_server)
        .await;

    mock_server
}
```

---

## Мониторинг и отладка

### Логирование

#### Client Logs
**Файл**: `C:\Program Files\data-exporter\logs\service.log`

**Формат**:
```
2026-01-12T14:30:00.123456Z  INFO data_exporter::processor: Batch started batch_id=abc-123
2026-01-12T14:30:15.234567Z  INFO data_exporter::processor: File processed file=data.dbf
2026-01-12T14:30:30.345678Z ERROR data_exporter::processor: File failed file=corrupted.dbf error="ConversionError"
2026-01-12T14:35:00.456789Z  INFO data_exporter::processor: Batch COMPLETED WITH WARNINGS batch_id=abc-123 warnings=1 completed=9
```

**Уровни логирования**:
- `ERROR`: Ошибки обработки файлов, критичные ошибки
- `WARN`: Warnings, retry attempts
- `INFO`: Успешные операции, batch lifecycle
- `DEBUG`: Детальная информация для отладки

#### Server Logs

**Рекомендуемые поля**:
```json
{
  "timestamp": "2026-01-12T14:30:00Z",
  "level": "info",
  "service": "batch-api",
  "batch_id": "abc-123",
  "device_id": "device-456",
  "action": "batch_started",
  "client_version": "1.0.0"
}
```

### Метрики

#### Client Metrics (Потенциальные)
- Batch processing duration
- Files processed per batch
- Error rate by type
- Network request latency
- Disk space usage

#### Server Metrics
- Batches started/completed/failed per hour
- Error reports by type
- Client version distribution
- Upload success rate
- API response times

### Troubleshooting Guide

#### Проблема: Batch не стартует

**Симптомы**:
```
ERROR Failed to start batch: 401 Unauthorized
```

**Диагностика**:
1. Проверить JWT token expiration
2. Проверить credentials в config.toml
3. Проверить server auth endpoint

**Решение**:
```bash
# Проверить config
type "C:\Program Files\data-exporter\config.toml"

# Проверить логи
type "C:\Program Files\data-exporter\logs\service.log" | findstr ERROR

# Переустановить с новыми credentials
data-exporter.exe uninstall
data-exporter.exe install --username <user> --password <pass>
```

#### Проблема: Files не загружаются

**Симптомы**:
```
ERROR Failed to upload file: 422 Unprocessable Entity
```

**Диагностика**:
1. Проверить формат файла (должен быть GZIP)
2. Проверить размер файла (limit на сервере)
3. Проверить server upload endpoint

**Решение**:
```bash
# Проверить размер файлов
dir "C:\Windows\SystemTemp\dbf_export_*.csv"

# Проверить GZIP compression
# (файлы должны удаляться после upload, проверить логи)
```

#### Проблема: Network share недоступен

**Симптомы**:
```
ERROR Failed to load config: Source directory does not exist: "\\server\share\data"
INFO Retrying in 1 minute...
```

**Диагностика**:
1. Проверить network connectivity: `ping server`
2. Проверить доступ к share: `dir \\server\share`
3. Проверить permissions

**Решение**:
- Service автоматически retry (1,2,4,8,16 min, then hourly)
- Если проблема persist, проверить network/permissions
- Service продолжит retry до успеха или manual stop

---

## Контрольный список для production deployment

### Pre-Deployment

- [ ] Все server endpoints реализованы и протестированы
- [ ] Database schema создана и мигрирована
- [ ] API authentication работает
- [ ] SSL/TLS сертификаты настроены
- [ ] Firewall rules настроены для HTTPS

### Client Configuration

- [ ] `config.toml` с production API URL
- [ ] Valid credentials настроены
- [ ] Source directory path корректен
- [ ] Cron schedule настроен правильно
- [ ] HTTPS-only mode включен

### Monitoring

- [ ] Server logging настроен
- [ ] Error alerting настроен
- [ ] Dashboard для batch monitoring
- [ ] Metrics collection настроена

### Testing

- [ ] End-to-end test выполнен успешно
- [ ] Error scenarios протестированы
- [ ] Network failure recovery протестирован
- [ ] Batch lifecycle transitions протестированы

### Documentation

- [ ] API documentation обновлена
- [ ] Troubleshooting guide создан
- [ ] Operational runbook создан
- [ ] Client installation guide обновлен

---

## Changelog

### Version 1.0.0 (2026-01-12)
- ✅ Initial implementation complete
- ✅ Auth client with auto-renewal
- ✅ Batch lifecycle management
- ✅ File upload with retry
- ✅ Error reporting with fallback
- ✅ Config hot-reload
- ✅ VSS locked file handling
- ✅ Service startup retry for network share
- ✅ Error classification (critical vs warnings)
- ✅ Detailed error chain reporting

---

## Appendix

### API Endpoint Summary

| Endpoint | Method | Auth | Client Impl | Server Impl |
|----------|--------|------|-------------|-------------|
| `/api/auth/token` | POST | Basic | ✅ | ⚠️ Required |
| `/api/v1/device/batches/start` | POST | Bearer | ✅ | ⚠️ Required |
| `/api/v1/device/batches/{id}/complete` | POST | Bearer | ✅ | ⚠️ Required |
| `/api/v1/device/batches/{id}/complete-with-warnings` | POST | Bearer | ✅ | ⚠️ Required |
| `/api/v1/device/batches/{id}/fail` | POST | Bearer | ✅ | ⚠️ Required |
| `/api/v1/device/batches/{id}/cancel` | POST | Bearer | ✅ | 🔵 Optional |
| `/api/files/upload` | POST | Bearer | ✅ | ⚠️ Required |
| `/api/errors/report` | POST | Bearer | ✅ | ⚠️ Required |

**Legend**:
- ✅ Implemented
- ⚠️ Required for production
- 🔵 Optional/future

### Error Type Mapping

| Client Error Type | HTTP Status | Server Action |
|------------------|-------------|---------------|
| `FileReadError` | - | Log, track file corruption rate |
| `EncodingError` | - | Log, suggest encoding settings |
| `ConversionError` | - | Log, analyze DBF format issues |
| `CompressionError` | - | Log, check disk space |
| `UploadError` | 4xx/5xx | Log, retry if 5xx |
| `NetworkError` | - | Log, check connectivity |
| `DirectoryInaccessible` | - | Alert, check permissions |
| `AuthenticationError` | 401 | Alert, check credentials |
| `ConfigurationError` | - | Alert, check config syntax |
| `VssError` | - | Log, check VSS service |

---

**Документ подготовлен**: 2026-01-12
**Версия клиента**: 1.0.0
**Автор**: Data Exporter Team
**Статус**: Ready for server-side implementation
