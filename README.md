# Data Exporter Service

[![Windows Build](https://github.com/quantum-soft-dev/dbf-uploader/workflows/Windows%20Build/badge.svg)](https://github.com/quantum-soft-dev/dbf-uploader/actions)
[![Release](https://github.com/quantum-soft-dev/dbf-uploader/workflows/Release/badge.svg)](https://github.com/quantum-soft-dev/dbf-uploader/releases)

Windows сервис для автоматического экспорта DBF файлов в CSV, сжатия в gzip и загрузки на облачный сервер по расписанию.

> **⚠️ Версия 2.0 - Важные изменения**
> Версия 2.0 использует новый middleware batch protocol с аутентификацией через site credentials.
> Миграция с v1.0: используйте встроенный мастер миграции (см. раздел "Миграция с v1.0").
> Основные изменения:
> - Аутентификация: `domain` + `client_secret` вместо `username` + `password`
> - Batch protocol: загрузка файлов группами с отслеживанием состояния
> - JWT токены с автоматическим обновлением
> - Новая структура конфигурации

## 🚀 Возможности

- **Автоматизация**: Экспорт по расписанию (cron)
- **Обработка**: DBF → CSV (UTF-8) → gzip
- **Загрузка**: Batch protocol с JWT аутентификацией и отслеживанием состояния
- **Аутентификация**: Site credentials (domain + client_secret)
- **Кодировки**: Автоопределение CP866, Windows-1251, UTF-8
- **Мониторинг**: Отправка ошибок на сервер + локальное логирование
- **Надёжность**: Обработка заблокированных файлов, retry логика, автоматическое обновление токенов
- **Конфигурация**: Hot-reload без перезапуска сервиса

## 📋 Требования

- **ОС**: Windows 10+ / Windows Server 2016+
- **Архитектура**: x86_64
- **Права**: Администратор (для установки сервиса)
- **Подключение**: HTTPS доступ к API серверу

## 🔧 Установка

### Скачать релиз

Скачайте последнюю версию из [Releases](https://github.com/quantum-soft-dev/dbf-uploader/releases).

### Новая установка (v2.0)

```powershell
# Распакуйте архив
Expand-Archive -Path data_exporter-v2.0.0-windows-x86_64.zip

# Запустите интерактивный установщик (требуются права администратора)
.\data_exporter.exe install
```

Мастер установки запросит:
- **Domain** - домен вашего сайта (например, store-01.example.com)
- **Client Secret** - секретный ключ для аутентификации (UUID формат)
- **Source Directory** - путь к папке с DBF файлами
- **API URL** - адрес middleware API (HTTPS обязателен)
- **Crontab** - расписание в формате cron
- **Encoding** - fallback кодировка (по умолчанию CP866)

### Миграция с v1.0

Если у вас установлена версия 1.x:

```powershell
# Распакуйте новую версию
Expand-Archive -Path data_exporter-v2.0.0-windows-x86_64.zip

# Запустите мастер миграции
.\data_exporter.exe migrate
```

Мастер миграции:
1. Обнаружит существующий `config.toml` v1.0
2. Запросит новые site credentials (domain + client_secret)
3. Сохранит старую конфигурацию как `config.toml.v1.backup`
4. Создаст новый `config.toml` v2.0 с сохранением всех остальных настроек
5. Перезапустит Windows сервис

⚠️ **Важно**: Убедитесь, что у вас есть:
- Domain вашего сайта
- Client Secret из middleware панели
- Резервная копия текущей конфигурации (создаётся автоматически)

### Проверка установки

```powershell
# Проверьте статус сервиса
sc query data-exporter

# Или через PowerShell
Get-Service -Name "data-exporter"
```

## ⚙️ Конфигурация

Файл конфигурации: `C:\Program Files\data-exporter\config.toml`

### Формат v2.0

```toml
version = "2.0"

[auth]
domain = "store-01.example.com"
client_secret = "a1b2c3d4-e5f6-7890-abcd-ef1234567890"

[api]
base_url = "https://middleware.example.com"

[source]
directory = "C:\\Data\\DBF"

[schedule]
crontab = "0 8,12,16,18 * * *"  # 8:00, 12:00, 16:00, 18:00

[encoding]
dbf_encoding = "CP866"  # Fallback кодировка для DBF файлов

[batch]
max_files_per_batch = 100
chunk_size_mb = 10
max_retries = 3
retry_delay_seconds = 30

[logging]
level = "info"
error_log_path = "error.log"
```

### Основные секции

- **[auth]** - Site credentials для аутентификации в middleware
  - `domain` - домен вашего сайта (обязательно)
  - `client_secret` - секретный ключ UUID формат (обязательно)

- **[api]** - Настройки API подключения
  - `base_url` - адрес middleware API, только HTTPS (обязательно)

- **[source]** - Исходные данные
  - `directory` - путь к папке с DBF файлами (обязательно)

- **[schedule]** - Расписание запуска
  - `crontab` - расписание в формате cron (обязательно)

- **[encoding]** - Настройки кодировки
  - `dbf_encoding` - fallback кодировка (по умолчанию CP866)

- **[batch]** - Настройки batch загрузки
  - `max_files_per_batch` - максимум файлов в одной группе (по умолчанию 100)
  - `chunk_size_mb` - размер chunk для больших файлов (по умолчанию 10 МБ)
  - `max_retries` - количество повторов при ошибке (по умолчанию 3)
  - `retry_delay_seconds` - задержка между повторами (по умолчанию 30 сек)

- **[logging]** - Настройки логирования
  - `level` - уровень логирования: trace, debug, info, warn, error (по умолчанию info)
  - `error_log_path` - путь к файлу локальных логов ошибок (по умолчанию error.log)

### Формат расписания (cron)

```
 ┌──── минута (0-59)
 │ ┌─── час (0-23)
 │ │ ┌─ день месяца (1-31)
 │ │ │ ┌ месяц (1-12)
 │ │ │ │ ┌ день недели (0-6, 0=воскресенье)
 │ │ │ │ │
 * * * * *
```

**Примеры**:
- `*/5 * * * *` - каждые 5 минут
- `0 */2 * * *` - каждые 2 часа
- `0 9 * * 1-5` - в 9:00 по будням
- `0 8,12,16,18 * * *` - в 8:00, 12:00, 16:00, 18:00

## 🔄 Обновление конфигурации

```powershell
# Отредактируйте config.toml
notepad "C:\Program Files\data-exporter\config.toml"

# Сохраните файл - изменения применятся автоматически
# при следующем запуске по расписанию
```

⚠️ **Важно**: Изменения применяются в начале следующего запланированного выполнения, не во время обработки.

## 🗑️ Удаление

```powershell
.\data_exporter.exe uninstall
```

## 📊 Мониторинг

### Логи сервиса

Просмотр через Event Viewer:
1. Откройте `eventvwr.msc`
2. Windows Logs → Application
3. Фильтр по источнику: "data-exporter"

### Локальные логи ошибок

При недоступности API сервера ошибки записываются в:
```
C:\Program Files\data-exporter\error.log
```

### Формат ошибок

```
[2025-10-05T14:30:00Z] ERROR: Failed to upload file
  Filename: reports\sales.dbf
  Error Type: UploadError
  Message: Network timeout
  Batch ID: abc-123-def-456
```

## 🏗️ Архитектура

### Компоненты v2.0

```
CLI (install/uninstall/migrate)
  ↓
Windows Service
  ↓
Cron Scheduler ← Config Watcher
  ↓
UploaderService
  ↓
AuthClient → TokenManager (JWT cache)
  ↓
BatchManager (batch lifecycle)
  ↓
Scanner → Converter → Compressor → Uploader
  ↓
Error Reporter (API + local fallback)
```

### Batch Protocol Workflow

1. **Authentication** - Получение JWT токена через site credentials
2. **Batch Start** - Создание новой группы загрузки (получение batch ID)
3. **File Upload** - Загрузка файлов multipart (chunked для больших файлов)
4. **Batch Complete** - Финализация группы (или Fail/Cancel при ошибках)
5. **Error Reporting** - Отправка ошибок на middleware (с fallback на локальный лог)

### Основные модули

- **auth** - Аутентификация через site credentials, JWT парсинг, автообновление токенов
- **batch** - Управление batch lifecycle (start/upload/complete/fail/cancel)
- **uploader** - Главный orchestrator обработки файлов
- **scanner** - Поиск DBF файлов в исходной папке
- **converter** - DBF → CSV конвертация с определением кодировки
- **compressor** - gzip сжатие CSV файлов
- **error** - Отправка ошибок на middleware API с fallback на локальный лог

## 🔐 Безопасность

- ✅ Только HTTPS соединения (HTTP разрешён только для localhost тестов)
- ✅ JWT токены кэшируются в памяти (не на диске)
- ✅ Автоматическое обновление JWT токенов до истечения
- ✅ Site credentials (domain + client_secret) вместо паролей
- ✅ Config.toml доступен только администраторам Windows
- ✅ Секретные данные не логируются
- ✅ Безопасный код (no unsafe Rust)

## 🧪 Разработка

### Требования

- Rust 1.82.0+
- Windows SDK

### Сборка

```powershell
# Debug
cargo build

# Release
cargo build --release

# Тесты
cargo test

# Линтинг
cargo clippy --all-targets --all-features
```

### CI/CD

Автоматическая сборка на Windows через GitHub Actions:
- Push → Build + Test
- Tag `v*.*.*` → Release на GitHub

Подробнее: [CI/CD документация](.github/CI_CD.md)

## 📝 Лицензия

См. файл [LICENSE](LICENSE)

## 🤝 Вклад

1. Fork репозиторий
2. Создайте feature branch (`git checkout -b feature/amazing-feature`)
3. Commit изменения (`git commit -m 'feat: add amazing feature'`)
4. Push в branch (`git push origin feature/amazing-feature`)
5. Откройте Pull Request

## 📚 Документация

- [Migration Guide](MIGRATION_GUIDE.md) - Миграция с v1.0 на v2.0
- [Configuration Guide](CONFIGURATION.md) - Полное описание конфигурации v2.0
- [Спецификация](specs/001-technical-specifications-data/spec.md)
- [План реализации](specs/001-technical-specifications-data/plan.md)
- [Quickstart Guide](specs/001-technical-specifications-data/quickstart.md)
- [CI/CD Guide](.github/CI_CD.md)
- [Quick Start](.github/QUICK_START.md)

## 🐛 Известные проблемы

### Windows Service не запускается
- Проверьте права администратора
- Убедитесь в корректности путей в config.toml
- Проверьте Event Viewer для деталей

### Файлы не загружаются
- Проверьте сетевое подключение к middleware API
- Убедитесь в корректности domain и client_secret в config.toml
- Проверьте, что middleware API использует HTTPS
- Убедитесь в валидности JWT токена (автообновляется автоматически)
- Проверьте `error.log` для деталей ошибок batch operations

### Кодировка некорректна
- Установите правильную fallback кодировку в config.toml
- DBF файлы без header encoding используют fallback

## 📞 Поддержка

- Issues: [GitHub Issues](https://github.com/quantum-soft-dev/dbf-uploader/issues)
- Email: support@example.com

## 🗺️ Roadmap

- [ ] Поддержка дополнительных форматов (XLS, XLSX)
- [ ] Web UI для мониторинга
- [ ] Метрики Prometheus
- [ ] Docker контейнер
- [ ] Linux/macOS версии
