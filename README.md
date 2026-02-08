# 🤖 ExpulsaBot

[![Rust](https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Docker](https://img.shields.io/badge/Docker-2496ED?style=for-the-badge&logo=docker&logoColor=white)](https://www.docker.com/)
[![Telegram](https://img.shields.io/badge/Telegram-2CA5E0?style=for-the-badge&logo=telegram&logoColor=white)](https://telegram.org/)
[![License](https://img.shields.io/badge/License-MIT-yellow.svg?style=for-the-badge)](LICENSE)

[![Build Status](https://img.shields.io/badge/build-passing-brightgreen?style=flat-square)](https://github.com/atareao/expulsabot)
[![Version](https://img.shields.io/badge/version-0.1.0-blue?style=flat-square)](https://github.com/atareao/expulsabot)
[![Rust Version](https://img.shields.io/badge/rust-1.70+-orange?style=flat-square)](https://www.rust-lang.org/)
[![Docker Image](https://img.shields.io/badge/docker-atareao/expulsabot-blue?style=flat-square)](https://hub.docker.com/r/atareao/expulsabot)

> 🛡️ **Bot de Telegram avanzado para protección anti-bot y verificación de usuarios**

ExpulsaBot es un bot de Telegram desarrollado en Rust que proporciona protección automática contra bots maliciosos y sistema de verificación inteligente para nuevos miembros de grupos.

---

## 🌟 **Características Principales**

### 🔒 **Protección Anti-Bot**

- ✅ **Expulsión automática** de bots no autorizados
- ✅ **Lista blanca configurable** para bots permitidos
- ✅ **Detección múltiple** (new_chat_members, new_chat_member, new_chat_participant)
- ✅ **Estadísticas detalladas** de bots expulsados

### 🎯 **Sistema de Desafíos**

- 🐧 **Desafíos con emojis de animales** (pingüino, ballena, cangrejo, zorro, foca, serpiente)
- ⏱️ **Timer configurable** (por defecto 2 minutos)
- 🔄 **Restricción temporal** durante el desafío
- 🧹 **Limpieza automática** de mensajes después de 30 segundos

### ⚙️ **Configuración Avanzada**

- 🌍 **Variable de entorno** para tratamiento de bots (`BAN_BOTS_DIRECTLY`)
- 🔔 **Notificaciones configurables** de expulsión
- 📊 **Comandos administrativos** completos
- 🕐 **Zona horaria Europe/Madrid**

---

## 🚀 **Inicio Rápido**

### **Docker Compose (Recomendado)**

1. **Crea tu archivo de configuración:**

```bash
cp .env.example .env
```

2. **Configura tu token de bot en `.env`:**

```env
TOKEN=tu_bot_token_aquí
CHALLENGE_DURATION_MINUTES=2
BAN_BOTS_DIRECTLY=true
MESSAGE_CLEANUP_DELAY_SECONDS=30
```

3. **Ejecuta con Docker Compose:**

```bash
docker compose up -d
```

### **Compilación Manual**

```bash
# Clonar repositorio
git clone https://github.com/atareao/expulsabot.git
cd expulsabot

# Compilar
cargo build --release

# Ejecutar
./target/release/expulsabot
```

---

## 📋 **Comandos Disponibles**

| Comando                 | Descripción                          | Ejemplo                  |
| ----------------------- | ------------------------------------ | ------------------------ |
| `/start`                | Iniciar el bot                       | `/start`                 |
| `/help`                 | Mostrar ayuda y configuración actual | `/help`                  |
| `/whitelist <bot_id>`   | Agregar bot a lista blanca           | `/whitelist 123456789`   |
| `/unwhitelist <bot_id>` | Remover bot de lista blanca          | `/unwhitelist 123456789` |
| `/stats`                | Ver estadísticas del grupo           | `/stats`                 |
| `/notify <on\|off>`     | Activar/desactivar notificaciones    | `/notify on`             |

---

## ⚙️ **Variables de Entorno**

| Variable                        | Descripción                     | Por Defecto     | Requerido |
| ------------------------------- | ------------------------------- | --------------- | --------- |
| `TOKEN`                         | Token del bot de Telegram       | -               | ✅        |
| `CHALLENGE_DURATION_MINUTES`    | Duración del desafío en minutos | `2`             | ❌        |
| `BAN_BOTS_DIRECTLY`             | Expulsar bots automáticamente   | `true`          | ❌        |
| `MESSAGE_CLEANUP_DELAY_SECONDS` | Tiempo para eliminar mensajes   | `30`            | ❌        |
| `TZ`                            | Zona horaria                    | `Europe/Madrid` | ❌        |
| `RUST_LOG`                      | Nivel de logging                | `INFO`          | ❌        |

---

## 🐳 **Docker**

### **Imagen Docker**

```bash
docker pull atareao/expulsabot:latest
```

### **Dockerfile Multi-etapa**

- 🏗️ **Builder**: Rust Alpine para compilación optimizada
- 🚀 **Runtime**: Alpine Linux minimalista (< 50MB)
- 🕐 **Timezone**: Configurado para Europe/Madrid
- 🔒 **Seguridad**: Usuario no-root preparado

---

## 📊 **Funcionalidades Avanzadas**

### **Modo de Tratamiento de Bots**

#### `BAN_BOTS_DIRECTLY=true` (Modo Estricto)

```
🤖 Bot detectado → ✅ Verificar lista blanca → ❌ Expulsar inmediatamente
```

#### `BAN_BOTS_DIRECTLY=false` (Modo Challenge)

```
🤖 Bot detectado → 🎯 Aplicar desafío → ❌ Expulsar si falla
```

### **Sistema de Limpieza Automática**

- **Éxito**: `"Juan ha pasado la verificación. ¡Bienvenido!"` → 🗑️ 30s
- **Fallo**: `"Ese no es el animal correcto."` → 🗑️ 30s
- **Timeout**: `"El usuario Juan fue expulsado..."` → 🗑️ 30s

---

## 🔧 **Desarrollo**

### **Tecnologías Utilizadas**

- **🦀 Rust 2021** - Lenguaje principal
- **⚡ Tokio** - Runtime asíncrono
- **🌐 Reqwest** - Cliente HTTP para Telegram API
- **📝 Serde** - Serialización JSON
- **🔍 Tracing** - Sistema de logging
- **🎲 Rand** - Generación aleatoria para desafíos

### **Estructura del Proyecto**

```
expulsabot/
├── src/
│   ├── main.rs           # Lógica principal del bot
│   └── telegram.rs       # Estructuras y API de Telegram
├── Cargo.toml           # Dependencias de Rust
├── Dockerfile           # Imagen Docker multi-etapa
├── compose.yml          # Configuración Docker Compose
├── .env.example         # Variables de entorno de ejemplo
└── README.md           # Este archivo
```

### **Compilar para Desarrollo**

```bash
# Compilación en modo debug
cargo build

# Ejecutar con logs detallados
RUST_LOG=debug cargo run

# Ejecutar tests
cargo test
```

---

## 🤝 **Contribuir**

¡Las contribuciones son bienvenidas! Por favor:

1. **Fork** el repositorio
2. **Crea** una rama para tu feature (`git checkout -b feature/nueva-funcionalidad`)
3. **Commit** tus cambios (`git commit -am 'Añadir nueva funcionalidad'`)
4. **Push** a la rama (`git push origin feature/nueva-funcionalidad`)
5. **Abre** un Pull Request

---

## 📝 **Licencia**

Este proyecto está bajo la Licencia MIT. Ver el archivo [LICENSE](LICENSE) para más detalles.

---

## 👨‍💻 **Autor**

**Lorenzo Carbonell** (@atareao)

- 🌐 [Website](https://atareao.es)
- 📧 [Email](mailto:lorenzo.carbonell.cerezo@gmail.com)
- 🐙 [GitHub](https://github.com/atareao)

---

## 🔗 **Enlaces Útiles**

- 📚 [Documentación de Telegram Bot API](https://core.telegram.org/bots/api)
- 🦀 [Documentación de Rust](https://doc.rust-lang.org/)
- 🐳 [Docker Hub](https://hub.docker.com/r/atareao/expulsabot)
- 📋 [Changelog](CHANGELOG.md)
- 🐛 [Reportar Bug](https://github.com/atareao/expulsabot/issues)

---

<div align="center">

**¡Dale una ⭐ si este proyecto te ha sido útil!**

[![GitHub stars](https://img.shields.io/github/stars/atareao/expulsabot?style=social)](https://github.com/atareao/expulsabot/stargazers)
[![GitHub forks](https://img.shields.io/github/forks/atareao/expulsabot?style=social)](https://github.com/atareao/expulsabot/network)

</div>
