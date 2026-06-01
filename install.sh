#!/usr/bin/env bash

# Salir inmediatamente si algún comando falla
set -e

echo "=== Instalando Bloqueador Web ==="

# 1. Compilar en release
echo "1. Compilando aplicación en modo Release..."
cargo build --release

# 2. Crear directorios necesarios
echo "2. Creando directorios del sistema..."
sudo mkdir -p /etc/bloqueador-web
sudo mkdir -p /usr/local/bin
sudo mkdir -p /usr/share/pixmaps
sudo mkdir -p /usr/share/applications

# 3. Copiar binario e icono
echo "3. Instalando binario y recursos..."
sudo cp target/release/Bloqueador-Web /usr/local/bin/bloqueador-web
sudo cp assets/icon.png /usr/share/pixmaps/bloqueador-web.png

# 4. Instalar servicio de Systemd
echo "4. Instalando servicio de Systemd..."
sudo cp bloqueador-web.service /etc/systemd/system/bloqueador-web.service
sudo systemctl daemon-reload

# 5. Habilitar e iniciar el servicio
echo "5. Habilitando e iniciando el Daemon de fondo..."
sudo systemctl enable bloqueador-web.service
sudo systemctl restart bloqueador-web.service

# 6. Instalar acceso directo en el sistema
echo "6. Instalando lanzador de escritorio..."
sudo cp bloqueador-web.desktop /usr/share/applications/bloqueador-web.desktop

# 7. Inicializar configuración vacía si no existe
if [ ! -f /etc/bloqueador-web/config.toml ]; then
    echo "7. Creando configuración inicial..."
    echo -e "autostart = false\nsites = []" | sudo tee /etc/bloqueador-web/config.toml > /dev/null
    sudo chmod 644 /etc/bloqueador-web/config.toml
fi

echo "=== ¡Instalación Completada con Éxito! ==="
echo "El daemon ya está corriendo de fondo como root."
echo "Ya podés buscar 'Bloqueador Web' en tu menú de aplicaciones de Omarchy y abrirlo."
