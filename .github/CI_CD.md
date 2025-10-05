# CI/CD для Data Exporter Service

## 🚀 Автоматизация сборки

Проект настроен с автоматической сборкой только под **Windows** с использованием GitHub Actions.

## 📋 Workflows

### 1. Сборка (build.yml)

**Запускается при:**
- Push в ветки: `main`, `develop`, или ветки спецификаций (`*-technical-specifications-*`)
- Pull Request в `main` или `develop`

**Что делает:**
```
✅ Проверка форматирования кода (cargo fmt)
✅ Линтинг с clippy
✅ Сборка в debug режиме
✅ Запуск всех тестов
✅ Сборка в release режиме
✅ Загрузка артефактов (data_exporter.exe)
```

**Артефакты:**
- `data_exporter-windows-x86_64` - исполняемый файл (хранится 30 дней)
- `data_exporter-windows-x86_64-debug` - отладочные символы (хранится 7 дней)

### 2. Release (release.yml)

**Запускается при:**
- Push тега версии: `v1.0.0`, `v2.1.3` и т.д.

**Что делает:**
```
✅ Сборка release версии
✅ Запуск тестов
✅ Создание ZIP архива
✅ Генерация SHA256 checksum
✅ Создание GitHub Release
```

**Результат:**
- GitHub Release с описанием
- ZIP архив: `data_exporter-v1.0.0-windows-x86_64.zip`
- SHA256 checksum файл

## 🔧 Создание релиза

### Шаг 1: Подготовка

```bash
# Убедитесь, что все тесты проходят
cargo test

# Проверьте форматирование
cargo fmt --all -- --check

# Запустите clippy
cargo clippy --all-targets --all-features
```

### Шаг 2: Создание тега

```bash
# Создайте тег с версией (следуйте semver)
git tag v1.0.0

# Отправьте тег в GitHub
git push origin v1.0.0
```

### Шаг 3: Автоматический релиз

GitHub Actions автоматически:
1. Соберёт Windows binary
2. Запустит тесты
3. Создаст release на GitHub
4. Загрузит архив и checksums

## 📥 Скачивание артефактов

### Из workflow run:
1. Откройте Actions → выберите workflow run
2. Scroll вниз до "Artifacts"
3. Скачайте `data_exporter-windows-x86_64.zip`

### Из release:
1. Откройте Releases в GitHub
2. Найдите нужную версию
3. Скачайте `data_exporter-v1.0.0-windows-x86_64.zip`

## 🔍 Проверка checksums

```powershell
# Windows PowerShell
$hash = Get-FileHash -Path data_exporter.exe -Algorithm SHA256
Get-Content data_exporter-v1.0.0-windows-x86_64.zip.sha256
# Сравните значения
```

## ⚙️ Локальная проверка перед push

Чтобы убедиться, что код пройдёт CI:

```powershell
# 1. Форматирование
cargo fmt --all

# 2. Линтинг
cargo clippy --all-targets --all-features -- -D warnings

# 3. Сборка
cargo build --verbose

# 4. Тесты
cargo test --verbose

# 5. Release сборка
cargo build --release --verbose
```

## 🐛 Troubleshooting

### Ошибка форматирования
```powershell
cargo fmt --all
git add .
git commit -m "Format code"
git push
```

### Ошибки clippy
```powershell
cargo clippy --fix --allow-dirty --allow-staged
git add .
git commit -m "Fix clippy warnings"
git push
```

### Тесты не проходят
```powershell
# Подробный вывод
cargo test -- --nocapture

# Конкретный тест
cargo test test_name -- --nocapture
```

## 📊 Статус сборки

Добавьте badge в README:

```markdown
![Build Status](https://github.com/YOUR_USERNAME/dbf-uploader/workflows/Windows%20Build/badge.svg)
```

## 🔐 Требования

- **Platform**: Windows 10+ или Windows Server 2016+
- **Rust**: 1.79.0 или выше
- **GitHub Actions**: Включены в репозитории

## 📝 Версионирование

Следуйте [Semantic Versioning](https://semver.org/):

- `v1.0.0` - Major release (breaking changes)
- `v1.1.0` - Minor release (new features, backwards compatible)
- `v1.1.1` - Patch release (bug fixes)

## 🚀 Roadmap CI/CD

Планируемые улучшения:

- [ ] Code coverage reporting
- [ ] Security scanning (cargo audit)
- [ ] Performance benchmarks
- [ ] Integration tests с mock server
- [ ] Auto-versioning
- [ ] Changelog generation
- [ ] Nightly builds

## 📞 Поддержка

При проблемах с CI/CD:
1. Проверьте логи в GitHub Actions
2. Запустите проверки локально
3. Создайте issue с описанием проблемы
