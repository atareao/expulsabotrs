# 🤖 ExpulsaBot

[![Rust](https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Docker](https://img.shields.io/badge/Docker-2496ED?style=for-the-badge&logo=docker&logoColor=white)](https://www.docker.com/)
[![Telegram](https://img.shields.io/badge/Telegram-2CA5E0?style=for-the-badge&logo=telegram&logoColor=white)](https://telegram.org/)
[![OpenObserve](https://img.shields.io/badge/OpenObserve-FF6B35?style=for-the-badge&logo=elasticsearch&logoColor=white)](https://openobserve.ai/)
[![Matrix](https://img.shields.io/badge/Matrix-000000?style=for-the-badge&logo=matrix&logoColor=white)](https://matrix.org/)
[![License](https://img.shields.io/badge/License-MIT-yellow.svg?style=for-the-badge)](LICENSE)

[![Build Status](https://img.shields.io/badge/build-passing-brightgreen?style=flat-square)](https://github.com/atareao/expulsabot)
[![Version](https://img.shields.io/badge/version-0.3.3-blue?style=flat-square)](https://github.com/atareao/expulsabot)
[![Rust Version](https://img.shields.io/badge/rust-1.70+-orange?style=flat-square)](https://www.rust-lang.org/)
[![Docker Image](https://img.shields.io/badge/docker-atareao/expulsabot-blue?style=flat-square)](https://hub.docker.com/r/atareao/expulsabot)

> 🛡️ **Bot de Telegram avanzado para protección anti-bot con desafíos de categorización inteligentes y monitoreo integral**

ExpulsaBot es un bot de Telegram desarrollado en Rust que proporciona protección automática contra bots maliciosos mediante desafíos de categorización de emojis, sistema de verificación avanzado para nuevos miembros de grupos, y registro completo de eventos en OpenObserve y Matrix.

---

## 🌟 **Características Principales**

### 🔒 **Protección Anti-Bot**

- ✅ **Expulsión automática** de bots no autorizados
- ✅ **Lista blanca configurable** para bots permitidos
- ✅ **Detección múltiple** (new_chat_members, new_chat_member, new_chat_participant)
- ✅ **Estadísticas detalladas** de bots expulsados

### 🎯 **Sistema de Desafíos de Categorización**

- 🎨 **Desafíos con categorización de emojis** (9 categorías: animales, comida, muebles, deportes, etc.)
- 🧩 **Formato intuitivo** (4 emojis de una categoría + 1 de otra diferente)
- 📝 **Preguntas gramaticalmente correctas** ("¿Cuál de estos NO es un animal?")
- 🎲 **177+ millones de combinaciones únicas** posibles
- ⚡ **Detección de bots por velocidad** (respuesta en menos de 1 segundo configurable)
- 🎯 **UUIDs únicos** para cada botón de respuesta
- ⏱️ **Timer configurable** (por defecto 2 minutos)
- 🔄 **Restricción temporal** durante el desafío
- 🧹 **Limpieza automática** de mensajes después de 30 segundos

### ⚙️ **Configuración Avanzada**

- 🌍 **Variable de entorno** para tratamiento de bots (`BAN_BOTS_DIRECTLY`)
- 🔔 **Notificaciones configurables** de expulsión
- ⚡ **Detección de velocidad de respuesta** (`MIN_RESPONSE_SECONDS`)
- 📊 **Comandos administrativos** completos
- 🕐 **Zona horaria Europe/Madrid**
- 🎛️ **Arquitectura modular** (main.rs, bot.rs, commands.rs)

### 📊 **Monitoreo y Analytics**

- 📈 **OpenObserve Integration** - Eventos JSON estructurados para análisis
- 💬 **Matrix Integration** - Notificaciones en tiempo real
- 📋 **Event Logging** - Registro completo de actividades de usuarios
- 🔍 **Estadísticas detalladas** de comportamiento de grupo

---

## 🎨 **Categorías de Desafíos**

El sistema incluye **9 categorías** perfectamente diferenciadas:

| Categoría                | Ejemplos de Emojis            | Pregunta                                       |
| ------------------------ | ----------------------------- | ---------------------------------------------- |
| 🐕 **Animales**          | 🐕 🐱 🐰 🐸 🦊 🐼 🐨 🦁 🐵 🐮 | "¿Cuál de estos NO es un animal?"              |
| 🍕 **Comida**            | 🍕 🍔 🍎 🍌 🍇 🥕 🍅 🥐 🧀 🥓 | "¿Cuál de estos NO es comida?"                 |
| 🪑 **Muebles y Decor.**  | 🪑 🛏️ 🛋️ 🪞 🕯️ 🏺 🖼️ 🕰️ 💡 🪟 | "¿Cuál de estos NO es un mueble o decoración?" |
| ⚽ **Deportes**          | ⚽ 🏀 🎾 🏈 ⚾ 🏐 🏓 🏸 🥊 🎱 | "¿Cuál de estos NO es un deporte?"             |
| 🚗 **Vehículos**         | 🚗 🚕 🚙 🚐 🚛 🚌 🚎 🏎️ 🚓 🚑 | "¿Cuál de estos NO es un vehículo?"            |
| ☀️ **Fenómenos Climát.** | ☀️ 🌙 ⭐ ☁️ ⛅ 🌧️ ⛈️ 🌩️ ❄️ 🌨️ | "¿Cuál de estos NO es un fenómeno climático?"  |
| 🔨 **Herramientas**      | 🔨 🔧 🪚 ⚒️ 🛠️ ⛏️ 🪓 🔩 ⚙️ 🪛 | "¿Cuál de estos NO es una herramienta?"        |
| 🌳 **Plantas**           | 🌳 🌲 🌴 🌵 🌿 🍀 🌺 🌸 🌼 🌻 | "¿Cuál de estos NO es una planta?"             |
| 🏠 **Edificios**         | 🏠 🏡 🏢 🏣 🏤 🏥 🏦 🏨 🏩 🏪 | "¿Cuál de estos NO es un edificio?"            |

**Ejemplos de desafíos generados:**

- **"¿Cuál de estos NO es comida?"** → 🍕 🍔 🥐 🧀 + 🚗 (vehículo)
- **"¿Cuál de estos NO es un animal?"** → 🐕 🐱 🦊 🐼 + 🌺 (planta)
- **"¿Cuál de estos NO es un vehículo?"** → 🚗 🚛 🚌 🏎️ + 🔨 (herramienta)

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
MIN_RESPONSE_SECONDS=1

# OpenObserve Integration (Opcional)
OPEN_OBSERVE_URL=tu_openobserve_url
OPEN_OBSERVE_TOKEN=tu_openobserve_token
OPEN_OBSERVE_INDEX=telegram_bot_events

# Matrix Integration (Opcional)
MATRIX_URL=tu_matrix_server
MATRIX_TOKEN=tu_matrix_access_token
MATRIX_ROOM=!roomId:server.com
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

| Comando                 | Descripción                           | Ejemplo                  |
| ----------------------- | ------------------------------------- | ------------------------ |
| `/start`                | Iniciar el bot                        | `/start`                 |
| `/help`                 | Mostrar ayuda y configuración actual  | `/help`                  |
| `/status`               | Ver estado y tiempo de funcionamiento | `/status`                |
| `/whitelist <bot_id>`   | Agregar bot a lista blanca            | `/whitelist 123456789`   |
| `/unwhitelist <bot_id>` | Remover bot de lista blanca           | `/unwhitelist 123456789` |
| `/stats`                | Ver estadísticas del grupo            | `/stats`                 |
| `/notify <on\|off>`     | Activar/desactivar notificaciones     | `/notify on`             |

---

## ⚙️ **Variables de Entorno**

| Variable                        | Descripción                     | Por Defecto     | Requerido |
| ------------------------------- | ------------------------------- | --------------- | --------- |
| `TOKEN`                         | Token del bot de Telegram       | -               | ✅        |
| `CHALLENGE_DURATION_MINUTES`    | Duración del desafío en minutos | `2`             | ❌        |
| `MIN_RESPONSE_SECONDS`          | Tiempo mínimo para respuesta    | `1`             | ❌        |
| `BAN_BOTS_DIRECTLY`             | Expulsar bots automáticamente   | `true`          | ❌        |
| `MESSAGE_CLEANUP_DELAY_SECONDS` | Tiempo para eliminar mensajes   | `30`            | ❌        |
| `TZ`                            | Zona horaria                    | `Europe/Madrid` | ❌        |
| `RUST_LOG`                      | Nivel de logging                | `INFO`          | ❌        |

### 📊 **Variables de OpenObserve** (Opcional)

| Variable             | Descripción                     | Ejemplo                           |
| -------------------- | ------------------------------- | --------------------------------- |
| `OPEN_OBSERVE_URL`   | URL de tu instancia OpenObserve | `https://openobserve.example.com` |
| `OPEN_OBSERVE_TOKEN` | Token de acceso OpenObserve     | `Basic dXNlcjpwYXNz...`           |
| `OPEN_OBSERVE_INDEX` | Índice donde guardar eventos    | `telegram_bot_events`             |

### 💬 **Variables de Matrix** (Opcional)

| Variable       | Descripción                    | Ejemplo                      |
| -------------- | ------------------------------ | ---------------------------- |
| `MATRIX_URL`   | Servidor Matrix (sin https://) | `matrix.example.com`         |
| `MATRIX_TOKEN` | Token de acceso Matrix         | `syt_dXNlcm5hbWU_xyz...`     |
| `MATRIX_ROOM`  | ID de sala Matrix              | `!roomId:matrix.example.com` |

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
🤖 Bot detectado → 🎨 Aplicar desafío de categorización → ❌ Expulsar si falla
```

### **Sistema de Limpieza Automática**

- **Éxito**: `"Juan ha pasado la verificación. ¡Bienvenido!"` → 🗑️ 30s
- **Fallo**: `"Esa no es la respuesta correcta."` → 🗑️ 30s
- **Bot detectado**: `"Respuesta demasiado rápida. Comportamiento de bot detectado."` → 🗑️ 30s
- **Timeout**: `"El usuario Juan fue expulsado..."` → 🗑️ 30s

### **Sistema de Monitoreo Integral**

#### 📊 **OpenObserve Analytics**

Cada evento de usuario se registra como JSON estructurado:

```json
{
  "user_id": 123456789,
  "user_name": "Juan Pérez",
  "group_id": -987654321,
  "group_name": "Mi Grupo de Telegram",
  "challenge_completed": true,
  "banned": false
}
```

#### 💬 **Matrix Notifications**

Mensajes en tiempo real enviados a Matrix:

- ✅ **Challenge exitoso**: `"el usuario Juan Pérez con id 123456789 si superó el challenge y no fue baneado del grupo Mi Grupo con id -987654321"`
- ❌ **Challenge fallido**: `"el usuario Juan Pérez con id 123456789 no superó el challenge y fue baneado del grupo Mi Grupo con id -987654321"`
- ⚡ **Bot detectado**: `"el usuario Juan Pérez con id 123456789 respondió demasiado rápido (500ms) y fue baneado del grupo Mi Grupo con id -987654321 por comportamiento de bot"`
- ⏰ **Timeout**: `"el usuario Juan Pérez con id 123456789 no superó el challenge y fue baneado del grupo Mi Grupo con id -987654321"`

---

## 🔧 **Desarrollo**

### **Tecnologías Utilizadas**

- **🦀 Rust 2021** - Lenguaje principal
- **⚡ Tokio** - Runtime asíncrono
- **🌐 Reqwest** - Cliente HTTP para APIs (Telegram, OpenObserve, Matrix)
- **📝 Serde** - Serialización JSON
- **🔍 Tracing** - Sistema de logging
- **🎲 Rand** - Generación aleatoria para desafíos de categorización
- **🆔 UUID** - Generación de identificadores únicos para botones
- **📊 OpenObserve** - Analytics y monitoreo de eventos
- **💬 Matrix** - Notificaciones en tiempo real

### **Estructura del Proyecto**

```
expulsabot/
├── src/
│   ├── main.rs              # Loop principal y manejo de eventos
│   ├── bot.rs               # Lógica de desafíos de categorización y gestión de bots
│   ├── commands.rs          # Manejo de comandos del bot
│   ├── telegram.rs          # Estructuras y API de Telegram
│   ├── openobserve.rs       # Integración con OpenObserve
│   ├── matrix.rs            # Integración con Matrix
│   └── challenge_tests.rs   # Tests unitarios completos
├── Cargo.toml              # Dependencias de Rust
├── Dockerfile              # Imagen Docker multi-etapa
├── compose.yml             # Configuración Docker Compose
├── .env.example            # Variables de entorno de ejemplo
└── README.md              # Este archivo
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
- � [OpenObserve Documentation](https://openobserve.ai/docs/)
- 💬 [Matrix.org](https://matrix.org/)
- �📋 [Changelog](CHANGELOG.md)
- 🐛 [Reportar Bug](https://github.com/atareao/expulsabot/issues)

---

<div align="center">

**¡Dale una ⭐ si este proyecto te ha sido útil!**

[![GitHub stars](https://img.shields.io/github/stars/atareao/expulsabot?style=social)](https://github.com/atareao/expulsabot/stargazers)
[![GitHub forks](https://img.shields.io/github/forks/atareao/expulsabot?style=social)](https://github.com/atareao/expulsabot/network)

</div>
