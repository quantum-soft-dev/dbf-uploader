# Data Exporter Service

[![Windows Build](https://github.com/quantum-soft-dev/dbf-uploader/workflows/Windows%20Build/badge.svg)](https://github.com/quantum-soft-dev/dbf-uploader/actions)
[![Release](https://github.com/quantum-soft-dev/dbf-uploader/workflows/Release/badge.svg)](https://github.com/quantum-soft-dev/dbf-uploader/releases)

Windows сервис для автоматического экспорта DBF файлов в CSV, сжатия в gzip и загрузки на облачный сервер по расписанию.

## 🚀 Возможности

- **Автоматизация**: Экспорт по расписанию (cron)
- **Обработка**: DBF → CSV (UTF-8) → gzip
- **Загрузка**: Multipart upload с JWT аутентификацией
- **Кодировки**: Автоопределение CP866, Windows-1251, UTF-8
- **Мониторинг**: Отправка ошибок на сервер + локальное логирование
- **Надёжность**: Обработка заблокированных файлов, retry логика
- **Конфигурация**: Hot-reload без перезапуска сервиса

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
.\data_exporter.exe install `
  --username your_username `
  --password your_password `
  --source-dir "C:\Data\DBF" `
  --crontab "0 8,12,16,18 * * *" `
  --api-url https://api.example.com `
  --encoding CP866
```

### Проверка установки

```powershell
# Проверьте статус сервиса
sc query data-exporter

# Или через PowerShell
Get-Service -Name "data-exporter"
```

## ⚙️ Конфигурация

Файл конфигурации: `C:\Program Files\data-exporter\config.toml`

```toml
[scheduler]
crontab = "0 8,12,16,18 * * *"  # 8:00, 12:00, 16:00, 18:00

[src]
source_dir = "C:\\Data\\DBF"

[credential]
username = "api_user"
password = "api_password"

[api]
base_url = "https://api.example.com"

[encoding]
dbf_encoding = "CP866"  # Fallback кодировка
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

```
CLI (install/uninstall)
  ↓
Windows Service
  ↓
Cron Scheduler ← Config Watcher
  ↓
Batch Orchestrator
  ↓
Scanner → Converter → Compressor → Uploader
  ↓
Error Reporter (API) / Logger (local)
```

## 🔐 Безопасность

- ✅ Только HTTPS соединения
- ✅ JWT токены в памяти (не на диске)
- ✅ Config.toml доступен только администраторам
- ✅ Пароли не логируются
- ✅ Безопасный код (no unsafe)

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

## 🐛 Известные проблемы

### Windows Service не запускается
- Проверьте права администратора
- Убедитесь в корректности путей в config.toml
- Проверьте Event Viewer для деталей

### Файлы не загружаются
- Проверьте сетевое подключение к API
- Убедитесь в валидности JWT токена
- Проверьте `error.log` для деталей

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
