 * [English](README.md)
 * Русский

# The Sculptor
[![CI](https://github.com/shiroyashik/sculptor/actions/workflows/ci.yml/badge.svg?branch=master)](https://github.com/shiroyashik/sculptor/actions/workflows/ci.yml)

Неофициальный бэкенд для Minecraft мода [Figura](https://github.com/FiguraMC/Figura).

Это полноценная замена официальной версии. Реализован весь функционал который вы можете использовать во время игры.

А также отличительной особенностью является возможность игры с сторонними провайдерерами аутентификации (такими как [Ely.By](https://ely.by/))

## Публичный сервер

[![Статус сервера](https://up.shsr.ru/api/badge/1/status?upLabel=Online&downLabel=Offline&label=Server+status)](https://up.shsr.ru/status/pub)

Я держу запущенным публичный сервер!

Вы можете использовать его если запуск собственного сервера затруднителен для вас.

Для подключения достаточно сменить **IP сервера Figura** в настройках Figura на адрес ниже:

> figura.shsr.ru

На сервере включена аутентификация через: Mojang(Microsoft) и [Ely.By](https://ely.by/)

## Запуск

Для его запуска вам понадобится настроенный обратный прокси-сервер.

Убедитесь, что используемый вами обратный прокси-сервер поддерживает WebSocket, а для HTTPS-соединений используются действительные сертификаты.

> [!WARNING]
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
# Клонируем пре-релиз
git clone https://github.com/shiroyashik/sculptor.git
# или из выбранного тега
git clone --depth 1 --branch v0.4.0 https://github.com/shiroyashik/sculptor.git
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

#### Сборка из `master` ветки

> [!IMPORTANT]
> Сборка Sculptor непосредственно из ветки `master` **не рекомендуется** для большинства пользователей. Эта ветка содержит предрелизный код, который активно разрабатывается и может содержать неработающие или нестабильные функции. Кроме того, использование ветки `master` может привести к проблемам с миграцией данных при обновлении до будущих стабильных релизов.
>
> Если вы все же решили использовать ветку `master`, пожалуйста, имейте в виду, что вы можете столкнуться с ошибками или некорректным поведением. Тем не менее ваши сообщения об ошибках высоко ценятся. Однако для более стабильной и надежной работы настоятельно рекомендую использовать **последний официальный релиз**.

## Вклад в развитие
![Спроси меня о чём угодно!](https://img.shields.io/badge/Ask%20me-anything-1abc9c.svg)
в
[![Telegram](https://badgen.net/static/icon/telegram?icon=telegram&color=cyan&label)](https://t.me/shiroyashik)
или
![Discord](https://badgen.net/badge/icon/discord?icon=discord&label)

Если у вас есть идем, нашли баг или хотите предложить улучшения
создавайте [issue](https://github.com/shiroyashik/sculptor/issues)
или свяжитесь со мной напрямую через Discord/Telegram (**@shiroyashik**).

Если вы Rust разработчик, буду рад вашим Pull Request'ам:

1. Форкните репу
2. Создайте новую ветку
3. Создайте PR!

Буду рад любой вашей помощи! ❤

## License

The Sculptor is licensed under [GPL-3.0](LICENSE)
