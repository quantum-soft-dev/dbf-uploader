# Быстрый старт - GitHub Actions

## 🚦 Проверка перед коммитом

```powershell
# Полная проверка (как в CI)
cargo fmt --all && `
cargo clippy --all-targets --all-features -- -D warnings && `
cargo test --verbose && `
cargo build --release
```

## 📦 Создание релиза

### 1. Обновите версию в Cargo.toml
```toml
[package]
name = "data_exporter"
version = "1.0.1"  # <- Обновите здесь
```

### 2. Закоммитьте изменения
```bash
git add Cargo.toml
git commit -m "chore: bump version to 1.0.1"
git push
```

### 3. Создайте и отправьте тег
```bash
git tag v1.0.1
git push origin v1.0.1
```

### 4. Дождитесь автосборки
- Откройте: https://github.com/YOUR_USERNAME/dbf-uploader/actions
- Дождитесь завершения "Release" workflow
- Проверьте: https://github.com/YOUR_USERNAME/dbf-uploader/releases

## 🔍 Проверка артефактов

### После workflow run:
```powershell
# Скачайте data_exporter-windows-x86_64.zip

# Распакуйте
Expand-Archive -Path data_exporter-windows-x86_64.zip -DestinationPath .

# Проверьте версию
.\data_exporter.exe --version

# Проверьте help
.\data_exporter.exe --help
```

## 🏷️ Соглашение об именовании

### Версии (Semantic Versioning):
- `v1.0.0` - Первый стабильный релиз
- `v1.1.0` - Новые функции (обратно совместимые)
- `v1.0.1` - Исправления ошибок
- `v2.0.0` - Breaking changes

### Ветки:
- `main` - стабильная версия
- `develop` - разработка
- `001-technical-specifications-data` - спецификации функций
- `feature/new-feature` - новые возможности
- `bugfix/issue-123` - исправления

## ⚡ Быстрые команды

```powershell
# Только форматирование
cargo fmt --all

# Только проверка форматирования (без изменений)
cargo fmt --all -- --check

# Только clippy
cargo clippy --all-targets --all-features

# Автофикс clippy (осторожно!)
cargo clippy --fix --allow-dirty

# Только тесты
cargo test

# Тесты с выводом
cargo test -- --nocapture

# Конкретный тест
cargo test test_batch_creation -- --nocapture

# Debug сборка
cargo build

# Release сборка
cargo build --release

# Очистка
cargo clean
```

## 📋 Checklist перед релизом

- [ ] Все тесты проходят локально
- [ ] Код отформатирован (`cargo fmt --all`)
- [ ] Нет warnings от clippy
- [ ] Версия обновлена в `Cargo.toml`
- [ ] CHANGELOG.md обновлен (если есть)
- [ ] README.md актуален
- [ ] Создан и отправлен git tag
- [ ] CI/CD прошёл успешно
- [ ] Release создан на GitHub
- [ ] Артефакты доступны для скачивания

## 🎯 Примеры workflow

### Feature branch → PR → Merge
```bash
# 1. Создайте ветку
git checkout -b feature/add-logging
git push -u origin feature/add-logging

# 2. Разработка
# ... ваш код ...
cargo test

# 3. Коммит
git add .
git commit -m "feat: add structured logging"
git push

# 4. Создайте PR на GitHub
# CI автоматически запустится

# 5. После approve - merge в develop
```

### Hotfix
```bash
# 1. Создайте ветку от main
git checkout main
git pull
git checkout -b hotfix/critical-bug

# 2. Исправление
# ... ваш код ...
cargo test

# 3. Коммит и push
git add .
git commit -m "fix: critical bug in auth"
git push -u origin hotfix/critical-bug

# 4. PR → main
# 5. После merge создайте тег:
git checkout main
git pull
git tag v1.0.2
git push origin v1.0.2
```

## 🛠️ Локальная отладка CI

Используйте [act](https://github.com/nektos/act) для локального запуска GitHub Actions:

```bash
# Установка (Windows)
choco install act-cli

# Запуск build workflow
act push

# Запуск release workflow
act push --eventpath event.json
```

## 📊 Мониторинг CI

### В GitHub UI:
1. **Actions tab** - все запуски
2. **Конкретный workflow** - история
3. **Artifacts** - скачать результаты

### Через GitHub CLI:
```bash
# Установка
winget install GitHub.cli

# Список workflows
gh workflow list

# Статус последнего run
gh run list --limit 1

# Просмотр логов
gh run view
```

## 🔗 Полезные ссылки

- [GitHub Actions Docs](https://docs.github.com/en/actions)
- [Rust CI/CD Best Practices](https://doc.rust-lang.org/cargo/guide/continuous-integration.html)
- [Semantic Versioning](https://semver.org/)
- [Conventional Commits](https://www.conventionalcommits.org/)
