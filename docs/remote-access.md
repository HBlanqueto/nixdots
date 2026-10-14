# Acceso remoto: ThinkPad → Mac mini M4

La ThinkPad (NixOS, gestionada por este flake) se conecta al Mac mini M4
(sin Nix: nada de nix-darwin ni home-manager, todo manual) a través de la
red local.

- **Compartir pantalla de macOS (VNC)**: método principal y de emergencia,
  con **Remmina** como cliente en la ThinkPad.
- **RustDesk**: alternativa más cómoda para el día a día.

Lado NixOS: `modules/common/remote-access.nix` (`nixdots.remoteAccess`) sólo
instala los paquetes `remmina` y `rustdesk-flutter`. Está activado sólo en
`hosts/ThinkPad-L14-Gen2/default.nix`; el host `UTM-aarch64` lo deja desactivado.

> No guardes en el repo contraseñas de VNC ni el ID de RustDesk. Todo eso se
> queda en cada máquina.

## Mac mini (manual)

### 1. Compartir pantalla (VNC)

1. **Ajustes del Sistema → General → Compartir**.
2. Activa **Pantalla remota** (*Screen Sharing*).
3. En el detalle (icono ⓘ), activa **Los visores VNC pueden controlar la
   pantalla con contraseña** y define una contraseña VNC. Añade los usuarios
   que puedan conectarse si usas "Sólo estos usuarios".
4. Deja **Inicio de sesión remoto** desactivado salvo que quieras SSH.

Anota la IP local del Mac en **Ajustes del Sistema → Red** (p. ej.
`192.168.x.y`); la usarás desde la ThinkPad.

### 2. Energía: que el Mac no se duerma

En **Ajustes del Sistema → Energía** (en Mac mini sólo hay corriente):

- **Evitar el reposo automático cuando la pantalla está apagada**: activado.
- **Activar por acceso a la red** (*Wake for network access*): activado.
- **Reanudar tras un corte de corriente** (*Start up automatically after a
  power failure*): activado, para el acceso de emergencia.

Equivalentes por terminal (más fiables entre versiones de macOS):

```shell
sudo pmset -a sleep 0          # sin reposo del sistema
sudo pmset -a womp 1           # activar por acceso a la red
sudo pmset -a autorestart 1    # arrancar tras corte de corriente
pmset -g                       # verificar
```

### 3. FileVault

Si FileVault está activo, **tras un reinicio no hay acceso remoto** hasta
desbloquear el disco en la pantalla. Para reinicios planificados usa:

```shell
sudo fdesetup authrestart
```

que reinicia guardando la autenticación para desbloquear el disco solo. Si
el Mac está a la espera de desbloqueo, sólo queda un reinicio local.

### 4. RustDesk (opcional)

1. Instala RustDesk desde <https://rustdesk.com> (arrastra la app a
   Aplicaciones).
2. **Ajustes del Sistema → Privacidad y seguridad**: concede **Grabación de
   pantalla** y **Accesibilidad** a RustDesk; añade también **Monitoreo de
   entrada** si macOS lo pide.
3. Configura una contraseña permanente y, si quieres, restringe el acceso a
   una sola vez. El **ID de RustDesk no se guarda en el repo**: se copia a
   mano a la ThinkPad.

## ThinkPad (lado flake)

1. Reconstruye:

   ```shell
   sudo nixos-rebuild switch --flake ~/nixdots#ThinkPad-L14-Gen2
   ```

   (`nixdots.remoteAccess.enable = true` trae los paquetes `remmina` y
   `rustdesk-flutter`.)

2. En Remmina, **Nueva conexión**:
   - **Protocolo**: VNC.
   - **Servidor**: la IP local del Mac (`192.168.x.y`), visible en
     **Ajustes del Sistema → Red**.
   - **Contraseña**: la contraseña VNC del Mac.
   Guarda el perfil; a partir de ahí conectar es un doble clic.

3. RustDesk (opcional): abre RustDesk, pega el ID del Mac y su contraseña.

## Verificación rápida

```shell
ping <ip-del-mac>                   # alcanzable en la red local
remmina                             # doble clic en el perfil VNC
```
