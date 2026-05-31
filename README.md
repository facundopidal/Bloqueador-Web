# Bloqueador Web

Una sencilla aplicación para Windows que se ejecuta en la bandeja del sistema (system tray) y permite bloquear y desbloquear sitios web modificando el archivo `hosts` del sistema.

## Características

- **Interfaz en la Bandeja del Sistema:** El programa se aloja discretamente en la bandeja del sistema, accesible a través de un icono.
- **Requiere Privilegios de Administrador:** Para poder modificar el archivo `hosts`, la aplicación verifica si se está ejecutando con permisos de administrador al inicio. Si no es así, muestra un error y se cierra.
- **Gestión de URLs:**
  - **Añadir URL:** Permite agregar una nueva URL a la lista de bloqueo a través de un simple cuadro de diálogo.
  - **Eliminar URL:** Las URLs bloqueadas se listan en un submenú. Un simple clic sobre una de ellas la elimina del bloqueo.
  - **Limpiar Todo:** Opción para eliminar todas las URLs bloqueadas de una sola vez.
- **Sistema de Logs:** Genera un archivo de registro (`.log`) en la misma carpeta del ejecutable para ayudar a diagnosticar problemas, especialmente cuando la aplicación se configura para arrancar con el sistema.

## ¿Cómo Funciona?

El programa utiliza una combinación de crates de Rust para lograr su funcionalidad:

1.  **`tray-icon` y `tao`:** Para crear el icono en la bandeja del sistema, el menú contextual y gestionar el bucle de eventos de la interfaz de usuario.
2.  **`is_elevated`:** Para comprobar si la aplicación se está ejecutando con los permisos de administrador necesarios.
3.  **`native-dialog`:** Para mostrar diálogos nativos de Windows (errores, confirmaciones).
4.  **Modificación del Archivo `hosts`:**
    - La lógica principal se encuentra en `src/hosts.rs`.
    - La aplicación lee el archivo `C:\Windows\System32\drivers\etc\hosts`.
    - Añade las URLs a bloquear dentro de un bloque delimitado por los marcadores `BEGIN BLOQUEADOR-WEB` y `END BLOQUEADOR-WEB`.
    - Cada URL bloqueada se traduce en una línea como `127.0.0.1 mi-sitio-web.com`.
    - Para evitar la pérdida de datos, se crea un backup (`.bak`) y se utiliza un archivo temporal (`.tmp`) para una escritura atómica.
5.  **`log` y `simplelog`:** Para registrar las acciones importantes y los errores en un archivo de texto, facilitando la depuración.

## Uso

### Compilación

Puedes compilar el proyecto usando Cargo:

```bash
# Para una versión de depuración (debug)
cargo build

# Para una versión optimizada de lanzamiento (release)
cargo build --release
```

El ejecutable se encontrará en `target/debug/` o `target/release/`.

### Ejecución

1.  Ejecuta `Bloqueador-Web.exe` **como Administrador**.
2.  El icono aparecerá en la bandeja del sistema.
3.  Haz clic derecho sobre el icono para acceder a las opciones:
    - **Añadir URL...:** Abre un cuadro de diálogo para que ingreses la URL a bloquear (ej: `youtube.com`).
    - **URLs Bloqueadas:** Muestra la lista de sitios bloqueados. Haz clic en uno para desbloquearlo.
    - **Limpiar Todo:** Elimina todas las reglas de bloqueo creadas por la aplicación.
    - **Salir:** Cierra la aplicación.

### Diagnóstico de Problemas (Logging)

Si la aplicación no se inicia o se comporta de manera inesperada (especialmente al configurarla para arrancar con Windows), revisa el archivo `Bloqueador-Web.log` que se crea en la misma carpeta que el archivo `.exe`. Este log contiene información sobre el flujo de ejecución y posibles errores.
