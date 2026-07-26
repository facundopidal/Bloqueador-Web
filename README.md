# Bloqueador Web

Una aplicación moderna y multiplataforma (Windows y Linux) escrita en Rust para bloquear sitios web distractivos y mejorar la productividad. Cuenta con una interfaz gráfica (GUI) desarrollada en **Slint**, un **daemon de fondo** autónomo, modos de bloqueo programado por horario y un sistema de fricción para evitar desbloqueos impulsivos.

---

## 🚀 Características Principales

- **Interfaz Gráfica Moderna (Slint):** Panel de control oscuro e intuitivo de 650x700px para gestionar tu lista de sitios y horarios de forma sencilla.
- **Soporte Multiplataforma:**
  - **Windows:** Integración con el archivo `hosts` del sistema, verificación de elevación de administrador y Programador de Tareas (`schtasks`).
  - **Linux:** Integración con `/etc/hosts`, elevación con `pkexec`, servicio `systemd` y lanzadores `.desktop`.
- **Modos de Bloqueo Flexibles:**
  - **Siempre Bloqueado (*Always*):** Bloquea el sitio las 24 horas del día.
  - **Por Horario (*Scheduled*):** Permite definir rangos horarios (ej. de `09:00` a `18:00` o nocturnos como de `22:00` a `06:00`). El sitio solo se bloqueará durante el horario configurado.
- **Fricción / Puzzle de Desbloqueo:**
  - Para evitar que desbloquees sitios por impulso durante tus horas de trabajo, eliminar un sitio requiere pasar por un **desafío de concentración**:
    - Cuenta regresiva obligatoria de 30 segundos.
    - Escritura exacta de la frase de compromiso: `"Prometo trabajar concentrado hoy"`.
- **Daemon de Fondo Autónomo (`--daemon`):**
  - Se ejecuta en segundo plano (polling cada 10 segundos), leyendo la configuración en `config.toml` y activando/desactivando bloqueos en el archivo `hosts` según la hora actual, sin necesidad de mantener la interfaz abierta.
- **Gestión de Inicio Automático (AutoStart):**
  - Botón integrado en la cabecera de la UI para activar o desactivar el arranque con el sistema.
- **Modificación Segura del Archivo `hosts`:**
  - Delimita sus reglas entre marcadores `# BEGIN BLOQUEADOR-WEB` y `# END BLOQUEADOR-WEB`.
  - Crea copias de seguridad automáticas (`hosts.bak`) antes de realizar cambios.

---

## 🛠️ ¿Cómo Funciona?

### Arquitectura de la Aplicación

El proyecto se divide en dos componentes principales gestionados desde un mismo binario:

1. **Interfaz Gráfica (GUI - Slint):**
   - Definida en `ui/appwindow.slint` y manejada por `src/main.rs`.
   - Permite agregar sitios, configurar horarios, cambiar el estado de inicio automático y resolver el puzzle de desbloqueo.
   - Guarda los cambios en `config.toml`.
2. **Daemon de Fondo (`--daemon`):**
   - Proceso ligero sin interfaz que se ejecuta al iniciar el sistema.
   - Consulta periódicamente `config.toml` e inspecciona la hora del sistema para modificar dinámicamente el archivo `hosts`.

### Persistencia y Archivos

- **Configuración (`config.toml`):**
  - **Windows:** Ubicado en la misma carpeta que el ejecutable.
  - **Linux:** Ubicado en `/etc/bloqueador-web/config.toml`.
- **Archivo `hosts`:**
  - **Windows:** `C:\Windows\System32\drivers\etc\hosts`
  - **Linux:** `/etc/hosts`

---

## 📦 Instalación y Uso

### Compilación General

Se requiere tener instalado el toolchain de **Rust** (cargo):

```bash
# Compilación para desarrollo
cargo build

# Compilación optimizada para producción (Release)
cargo build --release
```

---

### 🐧 Instalación en Linux

El proyecto incluye un script de instalación automatizado `install.sh` preparado para distribuciones Linux con `systemd`:

```bash
chmod +x install.sh
./install.sh
```

El script realiza lo siguiente:
1. Compila la aplicación en modo `--release`.
2. Copia el binario a `/usr/local/bin/bloqueador-web`.
3. Copia el icono a `/usr/share/pixmaps/bloqueador-web.png`.
4. Instala y habilita el servicio de systemd (`bloqueador-web.service`) para ejecutar el daemon de fondo como `root`.
5. Instala el acceso directo en el menú de aplicaciones (`bloqueador-web.desktop`).
6. Crea el archivo de configuración inicial en `/etc/bloqueador-web/config.toml`.

Para abrir la interfaz gráfica en Linux, simplemente busca **"Bloqueador Web"** en tu menú de aplicaciones o ejecuta:
```bash
bloqueador-web
```

---

### 🪟 Uso en Windows

1. Compila el proyecto con `cargo build --release`.
2. Ejecuta `target\release\Bloqueador-Web.exe` **como Administrador** (necesario para modificar el archivo `hosts`).
3. Para ejecutar el daemon en segundo plano:
   ```cmd
   Bloqueador-Web.exe --daemon
   ```
4. Si activas el **Inicio Automático** desde la UI, el programa creará una tarea en el **Programador de Tareas** (`schtasks`) para iniciarse automáticamente con privilegios al iniciar sesión.

---

## 🪵 Diagnóstico y Registros (Logs)

La aplicación genera registros detallados para facilitar la depuración (especialmente útil para el daemon de fondo):

- **Windows:** `Bloqueador-Web.log` en el directorio de la aplicación.
- **Linux:** `/var/log/bloqueador-web.log`.

