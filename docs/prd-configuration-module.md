# PRD: Модуль конфигурации
## dbf-uploader Data Exporter Service

**Версия:** 1.0
**Статус:** Draft
**Дата обновления:** 2026-01-24
**Автор:** Development Team

---

## 1. Краткое описание

Модуль конфигурации обеспечивает типобезопасную систему управления настройками для Windows-сервиса dbf-uploader. Он позволяет администраторам определять расписание, исходные директории, API-эндпоинты и учётные данные через TOML-файлы с поддержкой hot-reload и двух режимов аутентификации.

### Целевая аудитория
- Системные администраторы Windows Server
- IT-операционные команды
- Разработчики, интегрирующие сервис

### Ключевые возможности
- Загрузка и валидация конфигурации из TOML-файлов
- Hot-reload без перезапуска сервиса
- Двойная аутентификация (традиционная + Device Flow RFC 8628)
- Гибкая фильтрация файлов через glob-паттерны
- GUI-конфигуратор для Windows

---

## 2. Информация о документе

### История изменений

| Версия | Дата | Автор | Описание |
|--------|------|-------|----------|
| 1.0 | 2026-01-24 | Dev Team | Первоначальная версия PRD |

### Глоссарий

| Термин | Описание |
|--------|----------|
| **TOML** | Tom's Obvious Minimal Language — формат конфигурационных файлов |
| **Hot-reload** | Перезагрузка конфигурации без остановки сервиса |
| **Cron** | Формат выражений для планирования задач |
| **Device Flow** | OAuth 2.0 Device Authorization Grant (RFC 8628) для headless-устройств |
| **Glob** | Шаблоны для сопоставления имён файлов (*, ?, []) |

### Связанные документы
- `ARCHITECTURE.md` — архитектура проекта
- `README.md` — руководство пользователя
- `specs/001-data-exporter-service/spec.md` — техническая спецификация

---

## 3. Цели и метрики успеха

| Цель | Метрика | Статус |
|------|---------|--------|
| Типобезопасная загрузка конфигурации | Ноль паник при парсинге | Реализовано |
| Валидация всех полей | Все обязательные поля проверяются до запуска | Реализовано |
| Hot-reload | Изменения применяются без перезапуска | Реализовано |
| Двойная аутентификация | Поддержка traditional и device flow | Реализовано |
| Безопасное хранение credentials | Секреты не логируются | Реализовано |
| Покрытие тестами | >80% для модуля config | Реализовано (16 unit + 6 integration) |

---

## 4. Пользовательские истории

### US-CFG-001: Загрузка конфигурации из TOML

**Как** администратор системы,
**я хочу** загружать настройки из TOML-файла,
**чтобы** конфигурировать сервис без перекомпиляции.

**Критерии приёмки:**
- [x] AC1: Файл `config.toml` парсится в структуру `Config`
- [x] AC2: Ошибки парсинга возвращают понятные сообщения
- [x] AC3: Поддерживаются все 5 секций конфигурации

**Статус:** Реализовано
**Файлы:** `common/src/models/config.rs:120-127`

---

### US-CFG-002: Валидация полей конфигурации

**Как** администратор,
**я хочу** получать ошибки при некорректных значениях,
**чтобы** предотвратить запуск с неправильной конфигурацией.

**Критерии приёмки:**
- [x] AC1: Пустой crontab отклоняется
- [x] AC2: Несуществующая source_dir отклоняется
- [x] AC3: HTTP URL отклоняется при https_only=true
- [x] AC4: Некорректные glob-паттерны отклоняются

**Статус:** Реализовано
**Файлы:** `common/src/models/config.rs:143-206`

---

### US-CFG-003: Значения по умолчанию

**Как** администратор,
**я хочу** не указывать необязательные поля,
**чтобы** упростить конфигурацию.

**Критерии приёмки:**
- [x] AC1: `https_only` по умолчанию `true`
- [x] AC2: `dbf_encoding` по умолчанию `"CP866"`
- [x] AC3: `include_patterns` и `exclude_patterns` опциональны

**Статус:** Реализовано
**Файлы:** `common/src/models/config.rs:100-118`

---

### US-CFG-004: Hot-reload конфигурации

**Как** администратор,
**я хочу** изменять настройки без остановки сервиса,
**чтобы** минимизировать downtime.

**Критерии приёмки:**
- [x] AC1: ConfigWatcher отслеживает изменения файла
- [x] AC2: Debounce 2 секунды предотвращает множественные перезагрузки
- [x] AC3: Изменения применяются в начале следующего batch

**Статус:** Реализовано
**Файлы:** `service/src/config/watcher.rs`

---

### US-CFG-005: Традиционная аутентификация

**Как** администратор,
**я хочу** использовать account/username/password,
**чтобы** аутентифицироваться на API.

**Критерии приёмки:**
- [x] AC1: Формат username: `{account}_{username}`
- [x] AC2: Все три поля обязательны без device flow
- [x] AC3: Пустые поля отклоняются с понятной ошибкой

**Статус:** Реализовано
**Файлы:** `common/src/models/config.rs:39-92`

---

### US-CFG-006: Device Flow аутентификация (RFC 8628)

**Как** администратор headless-сервера,
**я хочу** использовать Device Authorization Flow,
**чтобы** безопасно получить credentials.

**Критерии приёмки:**
- [x] AC1: Секция `[credential.device]` с site_id, domain, client_secret
- [x] AC2: Если device присутствует, traditional credentials игнорируются
- [x] AC3: Пустые domain или client_secret отклоняются

**Статус:** Реализовано
**Файлы:** `common/src/models/config.rs:59-92`

---

### US-CFG-007: Фильтрация файлов glob-паттернами

**Как** администратор,
**я хочу** включать/исключать файлы по маске,
**чтобы** обрабатывать только нужные DBF.

**Критерии приёмки:**
- [x] AC1: `include_patterns` — whitelist (если указан)
- [x] AC2: `exclude_patterns` — blacklist (применяется после whitelist)
- [x] AC3: Case-insensitive matching
- [x] AC4: Невалидные паттерны отклоняются с указанием паттерна

**Статус:** Реализовано
**Файлы:** `common/src/models/config.rs:21-37, 190-203`

---

### US-CFG-008: GUI управление конфигурацией

**Как** администратор Windows,
**я хочу** использовать GUI для настройки,
**чтобы** не редактировать TOML вручную.

**Критерии приёмки:**
- [x] AC1: ConfigManager загружает config.toml или возвращает defaults
- [x] AC2: Сохранение создаёт директории при необходимости
- [x] AC3: TOML форматируется pretty-print

**Статус:** Реализовано
**Файлы:** `configurator/src/config_manager.rs`

---

### US-CFG-009: Retry при недоступности сетевого диска

**Как** администратор,
**я хочу** чтобы сервис ждал появления сетевой папки,
**чтобы** запускаться при старте системы до монтирования дисков.

**Критерии приёмки:**
- [x] AC1: Exponential backoff: 1, 2, 4, 8, 16 минут
- [x] AC2: После 5 попыток — hourly retries
- [x] AC3: Retry только для "Source directory does not exist"
- [x] AC4: Другие ошибки — немедленный fail

**Статус:** Реализовано
**Файлы:** `service/src/service/windows_service.rs`

---

### US-CFG-010: Graceful shutdown во время retry

**Как** администратор,
**я хочу** чтобы сервис останавливался корректно даже во время ожидания,
**чтобы** не блокировать перезагрузку системы.

**Критерии приёмки:**
- [x] AC1: Stop signal проверяется во время sleep
- [x] AC2: AtomicBool для межпоточной сигнализации
- [x] AC3: Немедленный выход при получении сигнала

**Статус:** Реализовано
**Файлы:** `service/src/service/windows_service.rs`

---

## 5. Функциональные требования

### 5.1 Схема конфигурации (TOML)

```toml
# Обязательная секция: расписание
[scheduler]
crontab = "0 */5 * * * *"  # 6-полей: сек мин час день месяц день_недели

# Обязательная секция: источник данных
[src]
source_dir = "C:\\data"              # Обязательно, директория должна существовать
include_patterns = ["*.dbf"]         # Опционально, whitelist
exclude_patterns = ["temp_*.dbf"]    # Опционально, blacklist

# Обязательная секция: учётные данные (один из вариантов)
[credential]
# Вариант A: Традиционная аутентификация
account = "company"                  # Обязательно без device flow
username = "admin"                   # Обязательно без device flow
password = "secret"                  # Обязательно без device flow

# Вариант B: Device Flow (RFC 8628)
# [credential.device]
# site_id = "uuid"                   # UUID сайта
# domain = "tenant_site"             # Обязательно
# client_secret = "cs_xxx"           # Обязательно

# Обязательная секция: API
[api]
base_url = "https://api.example.com" # Обязательно, http:// или https://
https_only = true                    # По умолчанию: true

# Обязательная секция: кодировка
[encoding]
dbf_encoding = "CP866"               # По умолчанию: "CP866"
```

### 5.2 Правила валидации

| Поле | Правило | Сообщение об ошибке |
|------|---------|---------------------|
| `scheduler.crontab` | Непустая строка | "Crontab expression cannot be empty" |
| `src.source_dir` | Директория существует | "Source directory does not exist: {path}" |
| `api.base_url` | Начинается с http:// или https:// | "API base URL must start with http:// or https://" |
| `api.base_url` | https:// при https_only=true | "API base URL must start with https:// when https_only is enabled" |
| `credential.account` | Непустая (без device flow) | "Account cannot be empty when not using device flow" |
| `credential.username` | Непустая (без device flow) | "Username cannot be empty when not using device flow" |
| `credential.password` | Непустая (без device flow) | "Password cannot be empty when not using device flow" |
| `credential.device.domain` | Непустая | "Device domain cannot be empty" |
| `credential.device.client_secret` | Непустая | "Device client_secret cannot be empty" |
| `src.include_patterns[*]` | Валидный glob | "Invalid include pattern '{pattern}': {error}" |
| `src.exclude_patterns[*]` | Валидный glob | "Invalid exclude pattern '{pattern}': {error}" |

### 5.3 Загрузка конфигурации

| ID | Требование |
|----|------------|
| FR-CFG-010 | Загрузка из файла `C:\Program Files\data-exporter\config.toml` |
| FR-CFG-011 | Парсинг TOML формата через `toml` crate |
| FR-CFG-012 | Валидация после парсинга |
| FR-CFG-013 | Возврат типизированной структуры `Config` |

### 5.4 Hot-Reload

| ID | Требование |
|----|------------|
| FR-CFG-020 | Отслеживание изменений файла через notify |
| FR-CFG-021 | Debounce 2 секунды |
| FR-CFG-022 | Перезагрузка в начале следующего batch |
| FR-CFG-023 | Fallback к текущему конфигу при ошибке reload |

### 5.5 Retry логика

| ID | Требование |
|----|------------|
| FR-CFG-030 | Retry только при "Source directory does not exist" |
| FR-CFG-031 | Exponential backoff: 1, 2, 4, 8, 16 минут |
| FR-CFG-032 | Hourly retry после 5 начальных попыток |
| FR-CFG-033 | Проверка stop signal во время sleep |
| FR-CFG-034 | Немедленный fail при других ошибках |

---

## 6. Нефункциональные требования

| ID | Категория | Требование | Цель | Статус |
|----|-----------|------------|------|--------|
| NFR-001 | Производительность | Загрузка конфига | < 100ms | Реализовано |
| NFR-002 | Производительность | Валидация | < 50ms | Реализовано |
| NFR-003 | Безопасность | Секреты не в логах | 100% | Реализовано |
| NFR-004 | Безопасность | HTTPS по умолчанию | Default true | Реализовано |
| NFR-005 | Надёжность | Graceful degradation | Кэш при ошибке | Реализовано |
| NFR-006 | Надёжность | Mutex poisoning recovery | Логирование + продолжение | Реализовано |
| NFR-007 | Качество | Покрытие тестами | >80% | Реализовано |

---

## 7. TDD методология

### 7.1 Философия тестирования

```
RED → GREEN → REFACTOR

1. Пишем падающий тест, описывающий ожидаемое поведение
2. Реализуем минимальный код для прохождения теста
3. Рефакторим, сохраняя тесты зелёными
```

### 7.2 Категории тестов

| Категория | Расположение | Количество | Цель |
|-----------|--------------|------------|------|
| Unit | `common/src/models/config.rs` (mod tests) | 16 | Отдельные функции |
| Unit | `service/src/config/watcher.rs` (mod tests) | 3 | ConfigWatcher |
| Integration | `tests/integration/config_reload_test.rs` | 6 | Межмодульное взаимодействие |

### 7.3 Матрица покрытия тестами

| Test ID | Функция | Описание | Статус |
|---------|---------|----------|--------|
| T001 | `Config::from_file` | Парсинг валидного TOML | Реализовано |
| T002 | `Config::validate` | Отклонение HTTP при https_only=true | Реализовано |
| T003 | `Config::validate` | Разрешение HTTP при https_only=false | Реализовано |
| T004 | `EncodingConfig` | Default encoding CP866 | Реализовано |
| T005 | `Config::validate` | Отклонение пустого crontab | Реализовано |
| T006 | `Config::validate` | Отклонение пустого account | Реализовано |
| T007 | `Config::validate` | Отклонение пустого username | Реализовано |
| T008 | `Config::validate` | Отклонение пустого password | Реализовано |
| T009 | `Config::validate` | Device flow: пустой domain | Реализовано |
| T010 | `Config::validate` | Device flow: пустой client_secret | Реализовано |
| T011 | `Config::validate` | Device flow: валидные credentials | Реализовано |
| T012 | `Config::validate` | Невалидный include pattern | Реализовано |
| T013 | `Config::validate` | Невалидный exclude pattern | Реализовано |
| T014 | `Config::validate` | Валидные glob patterns | Реализовано |
| T015 | `ConfigWatcher::new` | Создание watcher | Реализовано |
| T016 | `ConfigWatcher::has_changed` | Начальное состояние false | Реализовано |
| T017 | `ConfigWatcher::config_path` | Возврат пути | Реализовано |
| T018 | Integration | Детекция изменений файла | Реализовано |
| T019 | Integration | Обработка невалидного конфига | Реализовано |
| T020 | Integration | Сохранение patterns после reload | Реализовано |
| T021 | Integration | Backoff intervals | Реализовано |
| T022 | Integration | Retry только для directory_not_found | Реализовано |
| T023 | Integration | AtomicBool stop signal | Реализовано |

### 7.4 Шаблон для новых тестов

```rust
#[test]
fn test_[feature]_[scenario]_[expected_result]() {
    // Arrange: подготовка тестовых данных
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    let toml_content = format!(r#"
[scheduler]
crontab = "*/5 * * * *"
# ... остальные секции
    "#);

    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(toml_content.as_bytes()).unwrap();
    temp_file.flush().unwrap();

    // Act: выполнение тестируемого действия
    let result = Config::from_file(temp_file.path());

    // Assert: проверка результата
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("expected error message"));
}
```

### 7.5 Целевые показатели покрытия

| Метрика | Цель | Текущее значение |
|---------|------|------------------|
| Unit test coverage (config.rs) | >80% | ~85% |
| Integration test coverage | Все критические пути | 100% |
| Новые фичи | Тест ПЕРЕД кодом | Обязательно |
| Regression tests | Для каждого бага | Обязательно |

### 7.6 Правила TDD для команды

1. **Никакого кода без теста** — сначала тест, потом реализация
2. **Один тест — одна проверка** — атомарные assertions
3. **Понятные имена** — `test_[что]_[когда]_[результат]`
4. **Изоляция** — каждый тест независим, используем tempdir
5. **Быстрота** — unit тесты < 1 секунды
6. **CI обязателен** — все тесты должны проходить перед merge

---

## 8. Архитектура и интеграция

### 8.1 Диаграмма зависимостей модулей

```
+------------------+     +------------------+     +------------------+
|   configurator   |     |     common       |     |     service      |
|                  |     |                  |     |                  |
| +--------------+ |     | +--------------+ |     | +--------------+ |
| |ConfigManager |-+---->| |   Config     | |<----+-|ConfigWatcher | |
| +--------------+ |     | +--------------+ |     | +--------------+ |
|                  |     | |SchedulerCfg  | |     |                  |
|  UI bindings     |     | |SourceConfig  | |     | BatchScheduler   |
|                  |     | |CredentialCfg | |     |                  |
+------------------+     | |ApiConfig     | |     +------------------+
                         | |EncodingCfg   | |
                         | +--------------+ |
                         |                  |
                         | DeviceCredentials|
                         | validate()       |
                         | from_file()      |
                         | to_file()        |
                         +------------------+
```

### 8.2 Поток данных

```
1. Запуск сервиса:
   +-------------+     +-------------+     +----------+     +---------------+
   | config.toml |---->| from_file() |---->|validate()|---->|BatchScheduler |
   +-------------+     +-------------+     +----------+     +---------------+

2. Hot-Reload:
   +-------------+     +---------------+     +-------------+     +------------------+
   | File change |---->| ConfigWatcher |---->| Debounce 2s |---->| reload at batch  |
   +-------------+     +---------------+     +-------------+     +------------------+
                                                                        |
                                                                        v
                                                               +------------------+
                                                               | Arc<RwLock<Cfg>> |
                                                               +------------------+

3. GUI конфигурация:
   +---------------+     +----------+     +--------+     +---------------+
   | ConfigManager |---->|  load()  |---->|UI поля |---->| save_config() |
   +---------------+     +----------+     +--------+     +---------------+
```

### 8.3 Точки интеграции

| Компонент | Интеграция | Реализация |
|-----------|------------|------------|
| Windows Service | Загрузка конфига | `load_config_with_retry()` |
| Scheduler | Доступ к конфигу | `Arc<RwLock<Config>>` |
| Token Manager | Credentials | `TokenManager::new(&config)` |
| Batch Processor | Параметры | `run_batch(config, token_manager)` |
| GUI Configurator | CRUD операции | `ConfigManager::load/save()` |

---

## 9. Безопасность

### 9.1 Хранение credentials

| Аспект | Реализация |
|--------|------------|
| Формат хранения | TOML файл в защищённой директории |
| Расположение | `C:\Program Files\data-exporter\config.toml` |
| Права доступа | Администраторы Windows |
| Шифрование | Не реализовано (Future) |

### 9.2 Защита от утечек

| Угроза | Защита |
|--------|--------|
| Credentials в логах | `client_secret` не логируется |
| MITM атаки | `https_only=true` по умолчанию |
| Несанкционированный доступ | Windows file permissions |
| Устаревшие device codes | TTL 15 минут (RFC 8628) |

### 9.3 Threat Model

| Угроза | Вероятность | Влияние | Митигация |
|--------|-------------|---------|-----------|
| Credential exposure в логах | Низкая | Высокое | Не логировать секреты |
| Man-in-the-middle | Средняя | Высокое | HTTPS enforcement |
| Неавторизованное изменение конфига | Низкая | Среднее | Windows ACL |
| Брутфорс credentials | Средняя | Высокое | Rate limiting на сервере |

---

## 10. Примеры конфигурации

### 10.1 Минимальная конфигурация

```toml
[scheduler]
crontab = "0 0 8,12,16,18 * * *"

[src]
source_dir = "C:\\data"

[credential]
account = "mycompany"
username = "admin"
password = "secret"

[api]
base_url = "https://api.example.com"

[encoding]
```

### 10.2 Полная конфигурация с Device Flow

```toml
[scheduler]
# Каждые 5 минут
crontab = "0 */5 * * * *"

[src]
source_dir = "C:\\data\\dbf-files"
include_patterns = ["nsf*.dbf", "data_*.DBF"]
exclude_patterns = ["temp_*.dbf", "nsfcli.DBF", "NsfMod.dbf"]

[credential]
[credential.device]
site_id = "550e8400-e29b-41d4-a716-446655440000"
domain = "tenant_site-01"
client_secret = "cs_xxxxxxxxxxxxxxxxxxxxx"

[api]
base_url = "https://api.production.example.com"
https_only = true

[encoding]
dbf_encoding = "Windows-1255"
```

---

## 11. Статус реализации

### 11.1 Реализованные компоненты

| Компонент | Файл | Строк | Тесты |
|-----------|------|-------|-------|
| Config struct | `common/src/models/config.rs:5-12` | 8 | Да |
| SchedulerConfig | `config.rs:14-19` | 6 | Да |
| SourceConfig | `config.rs:21-37` | 17 | Да |
| CredentialConfig | `config.rs:39-92` | 54 | Да |
| ApiConfig | `config.rs:94-106` | 13 | Да |
| EncodingConfig | `config.rs:108-118` | 11 | Да |
| from_file() | `config.rs:120-127` | 8 | Да |
| to_file() | `config.rs:129-141` | 13 | Нет |
| validate() | `config.rs:143-206` | 64 | Да |
| ConfigWatcher | `service/src/config/watcher.rs` | 122 | Да |
| ConfigManager | `configurator/src/config_manager.rs` | 84 | Нет |

### 11.2 Gaps / Будущая работа

| Элемент | Приоритет | Описание |
|---------|-----------|----------|
| ConfigManager unit tests | Средний | Нет тестов для GUI config manager |
| to_file() unit tests | Низкий | Нет тестов для сериализации |
| Шифрование конфига | Низкий | Защита credentials на диске |
| Версионирование схемы | Низкий | Для миграции между версиями |
| Валидация cron выражений | Низкий | Проверка синтаксиса cron |

---

## 12. Миграция и версионирование

### 12.1 Текущая версия схемы

- **Версия:** 1.0
- **Формат:** Неявный (нет поля version в TOML)

### 12.2 Будущая стратегия миграции

```toml
# Предлагаемое добавление
[meta]
schema_version = "1.0"
```

### 12.3 Правила обратной совместимости

1. Новые опциональные поля должны иметь defaults
2. Устаревшие поля игнорируются с warning в логах
3. Удаление обязательных полей требует major version bump
4. Rename полей — deprecation + alias на 2 версии

---

## 13. Обработка ошибок

### 13.1 Типы ошибок

| Тип | Источник | Обработка |
|-----|----------|-----------|
| `ProcessingError::ConfigurationError` | Невалидный конфиг | Log + fail startup |
| TOML ParseError | Синтаксическая ошибка | Wrapped в ConfigurationError |
| IO Error | Файл не найден | Retry или fail |
| Validation Error | Невалидные значения | Немедленный fail |

### 13.2 Стратегия обработки

```
1. Ошибка парсинга TOML → Немедленный fail с сообщением
2. Source directory не существует → Retry с backoff
3. Невалидный URL → Немедленный fail
4. Невалидные credentials → Немедленный fail
5. Невалидный glob pattern → Немедленный fail с указанием паттерна
```

---

## 14. Приложения

### A. Расположение файлов

| Файл | Назначение |
|------|------------|
| `common/src/models/config.rs` | Core Config struct и валидация |
| `common/src/models/mod.rs` | Re-exports |
| `service/src/config/watcher.rs` | Hot-reload implementation |
| `service/src/config/mod.rs` | Module exports |
| `configurator/src/config_manager.rs` | GUI config management |
| `config.example.toml` | Example configuration |
| `tests/integration/config_reload_test.rs` | Integration tests |

### B. Справочник по cron выражениям

```
Формат: "секунды минуты часы день_месяца месяц день_недели"
        (6 полей для tokio-cron-scheduler)

Поля:
  секунды:      0-59
  минуты:       0-59
  часы:         0-23
  день_месяца:  1-31
  месяц:        1-12
  день_недели:  0-6 (0 = воскресенье)

Специальные символы:
  *     любое значение
  */n   каждые n единиц
  a,b   несколько значений
  a-b   диапазон

Примеры:
  "0 0 8,12,16,18 * * *"  - 8:00, 12:00, 16:00, 18:00 ежедневно
  "0 */5 * * * *"         - каждые 5 минут
  "0 0 */2 * * *"         - каждые 2 часа
  "0 30 9 * * 1-5"        - 9:30 по будням
```

### C. Справочник по glob паттернам

```
Синтаксис:
  *         любые символы (кроме /)
  ?         один символ
  [abc]     символ из набора
  [a-z]     символ из диапазона
  [!abc]    символ НЕ из набора

Примеры:
  *.dbf           все DBF файлы
  data_*.DBF      файлы начинающиеся с data_
  nsf[0-9].dbf    nsf0.dbf, nsf1.dbf, ... nsf9.dbf
  file?.dbf       file1.dbf, fileA.dbf (один символ)
  [a-z]*.dbf      файлы начинающиеся с буквы

Особенности:
  - Case-insensitive: *.DBF == *.dbf
  - Используется библиотека globset
  - Невалидные паттерны (напр. "[invalid") отклоняются
```

---

## История изменений документа

| Дата | Версия | Описание |
|------|--------|----------|
| 2026-01-24 | 1.0 | Первоначальная версия PRD |
