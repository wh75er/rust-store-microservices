# Template for RSOI lab 2 Miroservices

Оригинальное задание

## Сборка и запуск

```shell script
# Запуск PostgreSQL в контейнере
docker-compose up -d postgres
```

## Тестирование
Для проверки работоспособности системы используются скрипты Postman.
В папке [postman](postman) содержится [коллекция запросов](postman/postman-collection.json) к серверу и два enviroment'а:
* [local](postman/postman-local-environment.json);
* [heroku](postman/postman-heroku-environment.json).

Для автоматизированной проверки используется [GitHub Actions](.github/workflows/main.yml), CI/CD содержит шаги:
* сборка;
* деплой _каждого_ приложения на Heroku;
* прогон скриптов postman через newman для enviroment'а herkou.

## Запуск реализованной версии

Разворачивание производится на платформе Хироку. Можно использовать следующий скрипт в github actions:

```yaml
name: Build project
on: [push]
jobs:
  build:
    name: Build
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: nightly
          override: true
          components: rustfmt, clippy
      - name: Build app
        uses: actions-rs/cargo@v1
        with:
          command: check
          args: --release --all-features
      - name: Deploy Warranty Service
        uses: akhileshns/heroku-deploy@master
        with:
          heroku_api_key: <heroku api key>
          heroku_app_name: rocket-warranty-service
          heroku_email: wh75er@gmail.com
        env:
          HD_DATABASE_URL: <warranty database url>
          HD_ROCKET_DATABASES: '{pgdb={url=<warranty database url>}}'
          HD_SERVICE_NAME: warranty-service
      - name: Deploy Warehouse Service
        uses: akhileshns/heroku-deploy@master
        with:
          heroku_api_key: <heroku api key>
          heroku_app_name: rocket-warehouse-service
          heroku_email: wh75er@gmail.com
        env:
          HD_DATABASE_URL: <warehouse database url>
          HD_ROCKET_DATABASES: '{pgdb={url=<warehouse database url>}}'
          HD_SERVICE_NAME: warehouse-service
          HD_WARRANTY_HOST: https://rocket-warranty-service.herokuapp.com/
      - name: Deploy Order Service
        uses: akhileshns/heroku-deploy@master
        with:
          heroku_api_key: <heroku api key>
          heroku_app_name: rocket-order-service
          heroku_email: wh75er@gmail.com
        env:
          HD_RABBIT_MQ_HOST: <rabbit mq url>
          HD_DATABASE_URL: <orders database url>
          HD_ROCKET_DATABASES: '{pgdb={url=<orders database url>}}'
          HD_SERVICE_NAME: order-service
          HD_WARRANTY_HOST: https://rocket-warranty-service.herokuapp.com/
          HD_WAREHOUSE_HOST: https://rocket-warehouse-service.herokuapp.com/
      - name: Deploy Store Service
        uses: akhileshns/heroku-deploy@master
        with:
          heroku_api_key: <heroku api key>
          heroku_app_name: rocket-store-service
          heroku_email: wh75er@gmail.com
        env:
          HD_DATABASE_URL: <store database url>
          HD_ROCKET_DATABASES: '{pgdb={url=<store database url>}}'
          HD_SERVICE_NAME: store-service
          HD_WARRANTY_HOST: https://rocket-warranty-service.herokuapp.com/
          HD_WAREHOUSE_HOST: https://rocket-warehouse-service.herokuapp.com/
          HD_ORDER_HOST: https://rocket-order-service.herokuapp.com/
      - uses: actions/checkout@master
      - uses: matt-ball/newman-action@master
        with:
          collection: postman/postman-collectoin.json
          environment: postman/postman-heroku-environment.json
```
