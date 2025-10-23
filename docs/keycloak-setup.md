# Keycloak Setup Guide

Это руководство описывает базовую настройку Keycloak для окружения `data-forge-middleware`, работающего через `docker compose`. Оно покрывает сценарий разработки. Для production окружений дополнительно убедитесь в соблюдении требований информационной безопасности (пароли, HTTPS, резервное копирование и т.п.).

## 1. Требования

- Запущенный `docker compose` из корня репозитория `data-forge-middleware`.
- Доступ к браузеру с хоста.
- Установлены `docker` и `docker compose`.

## 2. Запуск сервисов

```powershell
docker compose up -d
```

Проверьте, что контейнер `dfm-keycloak` перешёл в статус `Running`. При необходимости смотрите в логи:

```powershell
docker compose logs -f keycloak
```

## 3. Доступ в административную консоль

- Откройте браузер и перейдите на `http://localhost:8081`.
- На странице входа укажите:
  - **Username:** `admin`
  - **Password:** `admin`
- После первого входа рекомендуется сразу сменить пароль администратора.

> Логин/пароль задаются в `docker-compose.yml` (`KEYCLOAK_ADMIN`, `KEYCLOAK_ADMIN_PASSWORD`). Для production используйте переменные окружения и не храните пароли в явном виде.

## 4. Проверка realm `dfm`

Файл `docker/keycloak/dfm-realm.json` импортируется автоматически (см. секцию `command` у сервиса `dfm-keycloak`), поэтому после входа в Keycloak вы увидите realm `dfm` с преднастроенными объектами:

- **Клиенты**
  - `dfm-backend` (конфигурация ресурсов backend-сервиса)
  - `dfm-ui` (пример клиентского приложения)
- **Пользователи**
  - `admin` (роль `ROLE_ADMIN`)
  - `user` (роль `ROLE_USER`)
- **Роли** и **группы** согласно настройкам realm’а.

Если нужно повторно импортировать realm:

1. Удалите realm `dfm` через админ-консоль (`Realm Settings → Delete`).
2. Перезапустите контейнер `dfm-keycloak`.

## 5. Обновление секретов клиента

Для взаимодействия backend с Keycloak используется клиент `dfm-backend`.

1. В меню `Clients` выберите `dfm-backend`.
2. В вкладке `Credentials` сгенерируйте новый секрет (`Regenerate Secret`).
3. Обновите секцию `SPRING_SECURITY_OAUTH2_…` и `JWT` в `docker-compose.yml` (или соответствующие переменные окружения), чтобы backend использовал новый секрет.
4. Перезапустите backend:

   ```powershell
   docker compose restart dfm-backend
   ```

## 6. Добавление новых пользователей или ролей

1. Перейдите в `Users → Add user`.
2. Задайте имя пользователя, включите флаг `Email Verified` (если требуется).
3. Во вкладке `Credentials` установите пароль.
4. Во вкладке `Role Mappings` назначьте роли (`ROLE_ADMIN`, `ROLE_USER` или кастомные роли).

## 7. Создание клиент-сайтов в middleware

1. Получите access token для администратора Keycloak:

   ```powershell
   $params = @{
     client_id  = 'dfm-backend'
     client_secret = 'dfm-backend-secret'
     grant_type = 'password'
     username   = 'admin'
     password   = 'admin'
   }

   $tokenResponse = Invoke-RestMethod `
     -Uri 'http://localhost:8081/realms/dfm/protocol/openid-connect/token' `
     -Method Post -Body $params
   $token = $tokenResponse.access_token
   ```

2. Создайте аккаунт в middleware:

   ```powershell
   $headers = @{ Authorization = "Bearer $token"; 'Content-Type' = 'application/json' }
   $accountBody = @{ email = 'store01@example.com'; name = 'Test Store 01' } | ConvertTo-Json

   Invoke-RestMethod `
     -Uri 'http://localhost:8080/api/admin/accounts' `
     -Method Post -Headers $headers -Body $accountBody
   ```

3. Создайте сайт и получите `clientSecret`:

   ```powershell
   $siteBody = @{ domain = 'store01.example.com'; displayName = 'Store 01 Primary' } | ConvertTo-Json

   $site = Invoke-RestMethod `
     -Uri "http://localhost:8080/api/admin/accounts/<ACCOUNT_ID>/sites" `
     -Method Post -Headers $headers -Body $siteBody

   $site.clientSecret  # сохраните это значение
   ```

Этими учётными данными будут пользоваться внешние клиенты/экспортёр. Значения, созданные выше, используем в конфиге приложения (см. инструкцию для экспортера).

## 8. Настройка HTTPS и доверенных хостов

Для продакшен-окружения:

- Настройте обратный прокси (Nginx/Traefik) перед Keycloak с валидным TLS-сертификатом.
- В `docker-compose.yml` измените `KC_PROXY`, `KC_HOSTNAME`, добавьте `KC_HOSTNAME_STRICT` при необходимости.
- Обновите `SPRING_SECURITY_OAUTH2_RESOURCESERVER_JWT_ISSUER_URI` и `JWK_SET_URI` в backend конфигурации, указывая HTTPS.

## 9. Резервные копии

- Периодически делайте экспорт realm через `Realm Settings → Export`.
- Для PostgreSQL (если используете внешний инстанс) настройте регулярные бэкапы.

---

С этими шагами Keycloak готов для локальной разработки вместе с `data-forge-middleware`. При миграции в продакшен дополнительно настройте безопасность (TLS, firewall, secret management, мониторинг).
