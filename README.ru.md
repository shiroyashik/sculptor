 * [English](README.md)
 * Русский

# The Sculptor
[![Push Dev](https://github.com/shiroyashik/sculptor/actions/workflows/dev-release.yml/badge.svg?branch=dev)](https://github.com/shiroyashik/sculptor/actions/workflows/dev-release.yml)

Неофициальный бэкенд для Minecraft мода [Figura](https://github.com/FiguraMC/Figura).

Это полноценная замена официальной версии. Реализован весь функционал который вы можете использовать во время игры.

А также отличительной особенностью является возможность игры с сторонними провайдерерами аутентификации (таких как [Ely.By](https://ely.by/))

## Публичный сервер

[![Статус сервера](https://up.shsr.ru/api/badge/1/status?upLabel=Online&downLabel=Offline&label=Server+status)](https://up.shsr.ru/status/pub)

Я держу запущенным публичный сервер!

Вы можете использовать его если запуск собственного сервера затруднителен для вас.

Для подключения достаточно сменить **Сервер IP** в настройках Figura на адрес ниже:

> figura.shsr.ru

На сервере включена аутентификация через: Mojang и [Ely.By](https://ely.by/)

По неконтролируемым мною причинам, сервер не доступен в некоторых странах

## Запуск

Для его запуска вам понадобится настроенный обратный прокси-сервер.

Убедитесь, что используемый вами обратный прокси-сервер поддерживает WebSocket, а для HTTPS-соединений используются действительные сертификаты.

> [!IMPORTANT]
> NGINX требует дополнительной настройки для работы с websocket!

### Docker

Как шаблон для начала можете использовать [docker-compose.example.yml](docker-compose.example.yml)

Предполагается, что вы будете использовать Traefik в качестве обратного прокси, если это так, раскомментируйте строки и добавьте Sculptor в сеть с Traefik.

Скопируйте [Config.example.toml](Config.example.toml) переименуйте в Config.toml и настройте по своему желанию.

Запустите! `docker compose up -d`

### Исполняемые файлы

Смотрите [прикреплённые архивы к релизам](https://github.com/shiroyashik/sculptor/releases/latest)

### Собираем из исходников

Для сборки потребуется предустановленный Rust

```sh
# Клонируем последний релиз
git clone https://github.com/shiroyashik/sculptor.git
# или из dev ветки
git clone --branch dev https://github.com/shiroyashik/sculptor.git
# Переходим в репу
cd sculptor
# Меняем имя конфиг файлу
cp Config.example.toml Config.toml
# Изменяем настройки (по желанию)
nano Config.toml
# Собираем с Release профилем для большей производительности
cargo build --release
# или запускаем прям из под cargo
cargo run --release
```

## Вклад в развитие

Если у вас есть идем, нашли баг или хотите предложить улучшения
создавайте [issue](https://github.com/shiroyashik/sculptor/issues)
или свяжитесь со мной напрямую через Discord/Telegram (@shiroyashik).

Если вы Rust разработчик, буду рад вашим Pull Request'ам:

1. Форкните репу
2. Создайте новую репу для вашего гения
3. Создайте PR!

Буду рад любой вашей помощи! ❤

#### Постскриптум

Ветка [“master”](https://github.com/shiroyashik/sculptor/tree/master) содержит код последнего релиза. А [“dev”](https://github.com/shiroyashik/sculptor/tree/dev) ветка дря разработки.

## License

The Sculptor is licensed under [GPL-3.0](LICENSE)
