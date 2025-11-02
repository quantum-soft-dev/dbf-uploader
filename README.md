# Data Exporter Service

[![Windows Build](https://github.com/quantum-soft-dev/dbf-uploader/workflows/Windows%20Build/badge.svg)](https://github.com/quantum-soft-dev/dbf-uploader/actions)
[![Release](https://github.com/quantum-soft-dev/dbf-uploader/workflows/Release/badge.svg)](https://github.com/quantum-soft-dev/dbf-uploader/releases)

Windows сервис для автоматического экспорта DBF файлов в CSV, сжатия в gzip и загрузки на облачный сервер по расписанию.

## 🚀 Возможности

- **Автоматизация**: Экспорт по расписанию (cron), автоматический запуск при старте системы
- **Обработка**: DBF → CSV (UTF-8) → gzip (в памяти для небольших файлов)
- **Загрузка**: Multipart upload с JWT аутентификацией
- **Кодировки**: Автоопределение CP866, Windows-1251, Windows-1255 (Hebrew), ISO-8859-8 (Hebrew), UTF-8
- **Мониторинг**: Ротация логов, детальные отчёты о выполнении батчей
- **Надёжность**: Обработка заблокированных файлов, retry логика, автоматическая очистка временных файлов
- **Производительность**: Обработка в памяти для файлов до 10 МБ, автоматический fallback на временные файлы
- **Мультиаккаунтность**: Поддержка нескольких аккаунтов через параметр `account`

## 📋 Требования

- **ОС**: Windows 10+ / Windows Server 2016+
- **Архитектура**: x86_64
- **Права**: Администратор (для установки сервиса)
- **Подключение**: HTTPS доступ к API серверу

## 🔧 Установка

### Скачать релиз

Скачайте последнюю версию из [Releases](https://github.com/quantum-soft-dev/dbf-uploader/releases).

### Установить сервис

```powershell
# Распакуйте архив
Expand-Archive -Path data_exporter-v1.0.0-windows-x86_64.zip

# Установите сервис (требуются права администратора)
# ВАЖНО: API URL должен включать версию API (например /api/v1)
.\data_exporter.exe install `
  --account "your-account-id" `
  --username "your_username" `
  --password "your_password" `
  --source-dir "C:\Data\DBF" `
  --crontab "0 8,12,16,18 * * *" `
  --api-url "https://api.example.com" `
  --encoding "Windows1255"

# Для локального тестирования с HTTP (небезопасно!)
.\data_exporter.exe install `
  --account "test-account" `
  --username "test_user" `
  --password "test_pass" `
  --source-dir "C:\Data\DBF" `
  --crontab "*/5 * * * *" `
  --api-url "http://localhost:8080" `
  --encoding "Windows1255" `
  --no-https
```

**Параметры установки**:
- `--account` - Идентификатор аккаунта (для мультиаккаунтности)
- `--username` - Имя пользователя для API
- `--password` - Пароль для API
- `--source-dir` - Директория с DBF файлами
- `--crontab` - Расписание в формате cron
- `--api-url` - URL API сервера
- `--encoding` - Fallback кодировка для DBF файлов (Windows1255, CP866, Windows1251, ISO8859-8, UTF8)
- `--no-https` - Разрешить HTTP подключения (только для тестирования!)

**Важно**: При аутентификации используется комбинированный username в формате `account_username` для обеспечения уникальности пользователей между аккаунтами.

### Проверка установки

```powershell
# Проверьте статус сервиса
sc query data-exporter

# Или через PowerShell
Get-Service -Name "data-exporter"

# Проверьте логи сервиса
Get-Content "C:\Program Files\data-exporter\logs\service.log" -Tail 50
```

**Примечание**: Сервис устанавливается с типом запуска "Automatic" и автоматически запускается после установки.

## ⚙️ Конфигурация

Файл конфигурации: `C:\Program Files\data-exporter\config.toml`

```toml
[scheduler]
crontab = "0 8,12,16,18 * * *"  # 8:00, 12:00, 16:00, 18:00

[src]
source_dir = "C:\\Data\\DBF"

[credential]
account = "your-account-id"     # Идентификатор аккаунта
username = "api_user"           # Имя пользователя
password = "api_password"       # Пароль

[api]
base_url = "https://api.example.com"
https_only = true  # Принудительно использовать только HTTPS (по умолчанию: true)
                   # ВНИМАНИЕ: Установка в false небезопасна! Используйте только для локального тестирования

[encoding]
dbf_encoding = "Windows1255"  # Fallback кодировка (по умолчанию - Hebrew)
```

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
# при следующем запуске по расписанию (без перезапуска сервиса)
```

⚠️ **Важно**: Изменения применяются в начале следующего запланированного выполнения, не во время обработки батча.

## 🗑️ Удаление

```powershell
.\data_exporter.exe uninstall
```

**Важно**: Сервис будет остановлен и удалён немедленно, но файлы установки будут удалены автоматически после завершения процесса (примерно через 2 секунды). Это нормальное поведение.

## 📊 Мониторинг

### Логи сервиса

Логи записываются в:
```
C:\Program Files\data-exporter\logs\service.log
```

**Функции логирования**:
- Автоматическая ежедневная ротация (создаётся новый файл в полночь)
- Старые логи переименовываются с датой: `service.log.2025-10-31`
- Детальная информация о каждом батче:
  - Время начала и окончания
  - Количество обработанных/ошибочных файлов
  - Заблокированные файлы (отложенные для повторной обработки)
  - Детальная диагностика ошибок API

**Просмотр логов**:
```powershell
# Последние 50 строк
Get-Content "C:\Program Files\data-exporter\logs\service.log" -Tail 50 -Wait

# Поиск ошибок
Select-String -Path "C:\Program Files\data-exporter\logs\service.log" -Pattern "ERROR"

# Логи за конкретную дату
Get-Content "C:\Program Files\data-exporter\logs\service.log.2025-10-31"
```

### Формат логов

```
2025-10-31T15:30:00.123456Z  INFO data_exporter: ========================================
2025-10-31T15:30:00.123457Z  INFO data_exporter: Scheduler woke up - starting batch execution
2025-10-31T15:30:00.123458Z  INFO data_exporter: Crontab: */5 * * * *
2025-10-31T15:30:00.123459Z  INFO data_exporter: Source: C:\data
2025-10-31T15:30:00.123460Z  INFO data_exporter: ========================================
2025-10-31T15:30:05.789012Z  INFO data_exporter: ========================================
2025-10-31T15:30:05.789013Z  INFO data_exporter: BATCH COMPLETED SUCCESSFULLY
2025-10-31T15:30:05.789014Z  INFO data_exporter: Batch ID: batch_abc123
2025-10-31T15:30:05.789015Z  INFO data_exporter: Files processed: 15
2025-10-31T15:30:05.789016Z  INFO data_exporter: Files failed: 0
2025-10-31T15:30:05.789017Z  INFO data_exporter: Files deferred (locked): 2
2025-10-31T15:30:05.789018Z  INFO data_exporter: Total files scanned: 17
2025-10-31T15:30:05.789019Z  INFO data_exporter: Duration: 5.67s
2025-10-31T15:30:05.789020Z  INFO data_exporter: ========================================
```

### Локальные логи ошибок

При недоступности API сервера ошибки также записываются в:
```
C:\Program Files\data-exporter\error.log
```

## 🏗️ Архитектура

```
CLI (install/uninstall)
  ↓
Windows Service (auto-start)
  ↓
Cron Scheduler ← Config Watcher (hot-reload)
  ↓
Batch Orchestrator
  ↓
Scanner → Converter (in-memory/temp) → Compressor (in-memory/temp) → Uploader
  ↓                   ↓
  ↓           ProcessingData (Drop для auto-cleanup)
  ↓
Error Reporter (API) / Logger (local + rotating)
```

**Ключевые компоненты**:

- **ProcessingData**: Абстракция для данных (в памяти или временный файл)
  - Автоматический fallback на временные файлы для больших файлов (>10 МБ)
  - Автоматическая очистка временных файлов через Drop trait

- **Converter**: DBF → CSV конвертация
  - В памяти для файлов <10 МБ
  - Временные файлы для больших DBF

- **Compressor**: CSV → GZIP сжатие
  - В памяти для небольших CSV
  - Потоковое сжатие для больших файлов

- **Batch Client**: Управление жизненным циклом батча на сервере
  - start_batch → upload files → complete_batch/fail_batch

## 🔐 Безопасность

- ✅ Только HTTPS соединения (по умолчанию)
- ✅ JWT токены в памяти (не на диске)
- ✅ Config.toml доступен только администраторам
- ✅ Пароли не логируются
- ✅ Безопасный код (no unsafe)
- ✅ Автоматическая очистка временных файлов (защита от утечки данных)

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

- [Спецификация](specs/001-technical-specifications-data/spec.md)
- [План реализации](specs/001-technical-specifications-data/plan.md)
- [Quickstart Guide](specs/001-technical-specifications-data/quickstart.md)
- [CI/CD Guide](.github/CI_CD.md)
- [Quick Start](.github/QUICK_START.md)

## 🐛 Устранение неполадок

### Windows Service не запускается
- Проверьте права администратора
- Убедитесь в корректности путей в config.toml
- Проверьте логи сервиса: `C:\Program Files\data-exporter\logs\service.log`
- Проверьте, что директория исходных файлов существует

### Файлы не загружаются
- Проверьте сетевое подключение к API
- Убедитесь в валидности учётных данных (account, username, password)
- Проверьте логи для деталей ошибок API
- Проверьте `error.log` если API недоступен

### Кодировка некорректна
- Установите правильную fallback кодировку в config.toml
- DBF файлы без header encoding используют fallback
- **Поддерживаемые кодировки**:
  - **Windows1255** (CP1255) - Hebrew (по умолчанию)
  - **ISO-8859-8** (ISO8859-8) - Hebrew (ISO)
  - **CP866** (IBM866) - DOS Cyrillic
  - **Windows1251** (CP1251) - Windows Cyrillic
  - **UTF8** (UTF-8) - Unicode

### Заблокированные файлы
- Сервис автоматически откладывает заблокированные файлы
- Повторная попытка выполняется в конце батча
- Если файл остаётся заблокированным - он пропускается с логированием

### Временные файлы не удаляются
- Проверьте права записи в `%TEMP%`
- Временные файлы создаются только для больших DBF (>10 МБ)
- Автоматическая очистка через Drop trait
- При сбое могут остаться файлы `dbf_export_*.csv` или `*.csv.gz` в `%TEMP%`

## 📞 Поддержка

- Issues: [GitHub Issues](https://github.com/quantum-soft-dev/dbf-uploader/issues)
- Email: support@example.com

## 🗺️ Roadmap

- [x] Автоматический запуск сервиса при старте системы
- [x] Ротация логов с автоматической очисткой
- [x] In-memory обработка для оптимизации производительности
- [x] Мультиаккаунтность через параметр account
- [ ] Поддержка дополнительных форматов (XLS, XLSX)
- [ ] Web UI для мониторинга
- [ ] Метрики Prometheus
- [ ] Docker контейнер
- [ ] Linux/macOS версии

## 📈 Последние изменения

### v1.0.0 (2025-11-01)
- ✨ Добавлен параметр `account` для мультиаккаунтности
- ✨ Автоматический запуск сервиса при старте системы
- ✨ Ежедневная ротация логов с автоматическим переименованием
- ✨ Детальные отчёты о выполнении батчей в логах
- ✨ In-memory обработка CSV и GZIP (до 10 МБ)
- ✨ Автоматический fallback на временные файлы для больших данных
- ✨ Автоматическая очистка временных файлов через Drop trait
- ✨ Упрощённый вывод CLI без технических деталей
- ✨ Расширенная диагностика ошибок API
- ✨ **Полная поддержка Hebrew**: Windows-1255 (по умолчанию) и ISO-8859-8
- 🐛 Исправлены все clippy warnings для соответствия стандартам Rust
- 🐛 Исправлена ошибка самоудаления при деинсталляции (Windows error 32)
- 🔧 Рефакторинг: InstallParams struct для уменьшения количества аргументов
- 🔧 Отложенное удаление файлов установки через batch файл
- 📚 Обновлена документация с новыми возможностями
