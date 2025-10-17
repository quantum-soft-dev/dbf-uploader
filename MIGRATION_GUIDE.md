# Migration Guide: v1.0 → v2.0

Руководство по миграции Data Exporter Service с версии 1.x на версию 2.0.

## 📋 Обзор изменений

### Критические изменения (Breaking Changes)

#### 1. Аутентификация
- **v1.0**: Username + Password
- **v2.0**: Site Credentials (Domain + Client Secret)

```toml
# v1.0
[credential]
username = "api_user"
password = "api_password"

# v2.0
[auth]
domain = "store-01.example.com"
client_secret = "a1b2c3d4-e5f6-7890-abcd-ef1234567890"
```

#### 2. Структура конфигурации
- **v1.0**: Плоская структура с секциями `[credential]`, `[api]`, `[src]`, `[scheduler]`, `[encoding]`
- **v2.0**: Расширенная структура с новыми секциями `[auth]`, `[source]`, `[schedule]`, `[batch]`, `[logging]`

#### 3. API Protocol
- **v1.0**: Простая загрузка файлов (multipart upload)
- **v2.0**: Batch protocol с lifecycle управлением (start → upload → complete/fail/cancel)

#### 4. JWT токены
- **v1.0**: Токены получались на каждую загрузку
- **v2.0**: Токены кэшируются и автоматически обновляются

### Новые возможности v2.0

- ✨ **Batch Protocol** - группировка файлов и отслеживание состояния загрузки
- ✨ **Token Management** - автоматическое обновление JWT токенов
- ✨ **Enhanced Error Reporting** - улучшенная отправка ошибок с fallback на локальный лог
- ✨ **Chunked Uploads** - поддержка загрузки больших файлов по частям
- ✨ **Retry Logic** - автоматические повторы при временных сбоях
- ✨ **Batch Configuration** - гибкая настройка размеров batch и chunk

## 🚀 Автоматическая миграция

### Предварительные требования

1. **Получите новые credentials**:
   - Войдите в панель middleware
   - Получите ваш **Domain** (например, `store-01.example.com`)
   - Сгенерируйте новый **Client Secret** (UUID формат)

2. **Создайте резервную копию**:
   ```powershell
   Copy-Item "C:\Program Files\data-exporter\config.toml" "C:\Backup\config.toml.backup"
   ```

3. **Скачайте v2.0**:
   - Перейдите на [Releases](https://github.com/quantum-soft-dev/dbf-uploader/releases)
   - Скачайте `data_exporter-v2.0.0-windows-x86_64.zip`
   - Распакуйте в отдельную папку

### Запуск мастера миграции

```powershell
# Перейдите в папку с новой версией
cd C:\path\to\data_exporter-v2.0.0

# Запустите мастер миграции (требуются права администратора)
.\data_exporter.exe migrate
```

### Процесс миграции

Мастер миграции выполнит следующие шаги:

1. **Обнаружение конфигурации v1.0**
   - Поиск `C:\Program Files\data-exporter\config.toml`
   - Проверка версии конфигурации

2. **Запрос новых credentials**
   ```
   Enter your site domain: store-01.example.com
   Enter your client secret: a1b2c3d4-e5f6-7890-abcd-ef1234567890
   ```

3. **Создание backup**
   - Старая конфигурация сохраняется как `config.toml.v1.backup`
   - Backup содержит полную копию v1.0 config

4. **Конвертация настроек**
   - Преобразование структуры конфигурации
   - Перенос всех существующих настроек:
     - `source_dir` → `[source].directory`
     - `crontab` → `[schedule].crontab`
     - `base_url` → `[api].base_url`
     - `dbf_encoding` → `[encoding].dbf_encoding`
   - Добавление значений по умолчанию для новых секций `[batch]` и `[logging]`

5. **Замена исполняемого файла**
   - Копирование нового `data_exporter.exe` в `C:\Program Files\data-exporter\`
   - Сохранение старого exe как `data_exporter.exe.v1.backup`

6. **Перезапуск сервиса**
   - Остановка Windows сервиса
   - Запуск с новой конфигурацией v2.0
   - Проверка успешного запуска

### Проверка миграции

```powershell
# Проверьте статус сервиса
Get-Service -Name "data-exporter"

# Проверьте логи в Event Viewer
eventvwr.msc
# Windows Logs → Application → Source: "data-exporter"

# Проверьте новую конфигурацию
notepad "C:\Program Files\data-exporter\config.toml"
```

Ожидаемый статус сервиса: `Running`

## 🔧 Ручная миграция

Если автоматическая миграция не подходит или вы хотите больше контроля:

### Шаг 1: Остановите сервис

```powershell
Stop-Service -Name "data-exporter"
```

### Шаг 2: Создайте резервную копию

```powershell
# Backup конфигурации
Copy-Item "C:\Program Files\data-exporter\config.toml" "C:\Backup\config.toml.v1.backup"

# Backup исполняемого файла
Copy-Item "C:\Program Files\data-exporter\data_exporter.exe" "C:\Backup\data_exporter.exe.v1.backup"
```

### Шаг 3: Обновите конфигурацию

Откройте `C:\Program Files\data-exporter\config.toml` и преобразуйте структуру:

**v1.0 config.toml:**
```toml
[scheduler]
crontab = "0 8,12,16,18 * * *"

[src]
source_dir = "C:\\Data\\DBF"

[credential]
username = "api_user"
password = "api_password"

[api]
base_url = "https://api.example.com"

[encoding]
dbf_encoding = "CP866"
```

**v2.0 config.toml:**
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
crontab = "0 8,12,16,18 * * *"

[encoding]
dbf_encoding = "CP866"

[batch]
max_files_per_batch = 100
chunk_size_mb = 10
max_retries = 3
retry_delay_seconds = 30

[logging]
level = "info"
error_log_path = "error.log"
```

### Шаг 4: Замените исполняемый файл

```powershell
# Скопируйте новый exe
Copy-Item "C:\path\to\data_exporter-v2.0.0\data_exporter.exe" "C:\Program Files\data-exporter\data_exporter.exe" -Force
```

### Шаг 5: Запустите сервис

```powershell
Start-Service -Name "data-exporter"

# Проверьте статус
Get-Service -Name "data-exporter"
```

## 🔄 Откат на v1.0

Если возникли проблемы, можно откатиться на v1.0:

### Автоматический откат

```powershell
# Остановите сервис
Stop-Service -Name "data-exporter"

# Восстановите v1.0 exe
Copy-Item "C:\Program Files\data-exporter\data_exporter.exe.v1.backup" "C:\Program Files\data-exporter\data_exporter.exe" -Force

# Восстановите v1.0 config
Copy-Item "C:\Program Files\data-exporter\config.toml.v1.backup" "C:\Program Files\data-exporter\config.toml" -Force

# Запустите сервис
Start-Service -Name "data-exporter"
```

### Проверка отката

```powershell
Get-Service -Name "data-exporter"
# Статус должен быть Running
```

## 📊 Сравнение конфигураций

### Таблица соответствия полей

| v1.0 | v2.0 | Изменение |
|------|------|-----------|
| `[credential].username` | `[auth].domain` | ⚠️ Изменён формат |
| `[credential].password` | `[auth].client_secret` | ⚠️ Изменён формат |
| `[api].base_url` | `[api].base_url` | ✅ Без изменений |
| `[src].source_dir` | `[source].directory` | 🔄 Переименовано |
| `[scheduler].crontab` | `[schedule].crontab` | 🔄 Переименовано |
| `[encoding].dbf_encoding` | `[encoding].dbf_encoding` | ✅ Без изменений |
| - | `[batch].*` | ✨ Новая секция |
| - | `[logging].*` | ✨ Новая секция |
| - | `version` | ✨ Новое поле |

### Значения по умолчанию для новых секций

```toml
[batch]
max_files_per_batch = 100      # Максимум файлов в одной группе
chunk_size_mb = 10              # Размер chunk для больших файлов (МБ)
max_retries = 3                 # Количество повторов при ошибке
retry_delay_seconds = 30        # Задержка между повторами (сек)

[logging]
level = "info"                  # trace, debug, info, warn, error
error_log_path = "error.log"    # Путь к файлу локальных логов
```

## 🐛 Устранение неполадок

### Ошибка: "Invalid configuration version"

**Причина**: Конфигурация не содержит поле `version = "2.0"`

**Решение**:
```toml
# Добавьте в начало config.toml:
version = "2.0"
```

### Ошибка: "Authentication failed: Invalid credentials"

**Причина**: Неверный domain или client_secret

**Решение**:
1. Проверьте domain в middleware панели
2. Сгенерируйте новый client_secret
3. Обновите `config.toml`:
```toml
[auth]
domain = "ваш-правильный-domain.example.com"
client_secret = "новый-client-secret-uuid"
```

### Ошибка: "API base URL must use HTTPS"

**Причина**: v2.0 требует HTTPS для production

**Решение**:
```toml
[api]
# Неправильно:
base_url = "http://middleware.example.com"

# Правильно:
base_url = "https://middleware.example.com"
```

### Ошибка: "Failed to start batch"

**Причина**: Проблемы с batch protocol на middleware

**Решение**:
1. Проверьте доступность middleware API
2. Убедитесь, что middleware поддерживает v2 batch protocol
3. Проверьте `error.log` для деталей:
```powershell
Get-Content "C:\Program Files\data-exporter\error.log" -Tail 20
```

### Сервис не запускается после миграции

**Диагностика**:
```powershell
# Проверьте Event Viewer
eventvwr.msc
# Windows Logs → Application → Source: "data-exporter"

# Проверьте синтаксис config.toml
notepad "C:\Program Files\data-exporter\config.toml"
```

**Решение**:
1. Убедитесь, что все обязательные поля заполнены
2. Проверьте формат путей (используйте двойные обратные слеши: `C:\\Data\\DBF`)
3. Если проблема не решена - откатитесь на v1.0 (см. раздел "Откат")

### Файлы не загружаются после миграции

**Диагностика**:
```powershell
# Проверьте локальные логи ошибок
Get-Content "C:\Program Files\data-exporter\error.log"
```

**Частые причины**:
1. **Неверные credentials** - проверьте domain и client_secret
2. **HTTPS требование** - убедитесь, что base_url использует HTTPS
3. **Middleware не поддерживает v2** - обновите middleware до версии с batch protocol
4. **Сетевые проблемы** - проверьте доступность middleware API

## 📞 Поддержка

Если возникли проблемы с миграцией:

1. **Проверьте логи**:
   - Event Viewer (Windows Logs → Application)
   - `C:\Program Files\data-exporter\error.log`

2. **Создайте Issue**:
   - [GitHub Issues](https://github.com/quantum-soft-dev/dbf-uploader/issues)
   - Приложите логи и конфигурацию (без секретов!)

3. **Email поддержка**:
   - support@example.com

## ✅ Чек-лист миграции

Используйте этот чек-лист для проверки миграции:

- [ ] Получены domain и client_secret из middleware панели
- [ ] Создана резервная копия `config.toml` и `data_exporter.exe`
- [ ] Запущен мастер миграции или выполнена ручная миграция
- [ ] Обновлена конфигурация на v2.0 формат
- [ ] Заменён исполняемый файл на v2.0
- [ ] Сервис успешно запущен (`Get-Service -Name "data-exporter"` → Running)
- [ ] Проверены логи в Event Viewer (нет критических ошибок)
- [ ] Выполнен тестовый запуск обработки файлов
- [ ] Проверена успешная загрузка на middleware
- [ ] Документированы изменения для команды

## 🗺️ Дополнительная документация

- [README.md](README.md) - Основная документация v2.0
- [CONFIGURATION.md](CONFIGURATION.md) - Подробное описание конфигурации
- [Спецификация v2](specs/001-technical-specifications-data/spec.md)
