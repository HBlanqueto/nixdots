# Acceso remoto: ThinkPad → Mac mini M4

La ThinkPad (NixOS, gestionada por este flake) se conecta al Mac mini M4
(sin Nix: nada de nix-darwin ni home-manager, todo manual) a través de
**Tailscale** como red privada, sin abrir puertos en ningún router.

- **Compartir pantalla de macOS (VNC)**: método principal y de emergencia,
  con **Remmina** como cliente en la ThinkPad.
- **RustDesk**: alternativa más cómoda para el día a día.

Lado NixOS: `modules/common/remote-access.nix` (`nixdots.remoteAccess`).
Está activado sólo en `hosts/thinkpad/default.nix`; el host `utm` lo deja
desactivado. La opción `nixdots.remoteAccess.trustTailscale` (por defecto
`false`) añadiría `tailscale0` a `networking.firewall.trustedInterfaces`; no
hace falta porque la ThinkPad sólo actúa como cliente.

> No guardes en el repo contraseñas de VNC, claves de Tailscale ni el ID de
> RustDesk. Todo eso se queda en cada máquina.

## Mac mini (manual)

### 1. Tailscale como servicio del sistema

Se instala la fórmula (daemon del sistema), no la app: así Tailscale arranca
al encender y está disponible **antes de iniciar sesión** (necesario para
VNC/SSH tras un reinicio).

```shell
brew install tailscale
sudo brew services start tailscale
sudo tailscale up
```

Comprueba con `tailscale status` y anota el nombre MagicDNS del Mac
(aparece a la izquierda, p. ej. `mac-mini.<tailnet>.ts.net`).

### 2. Compartir pantalla (VNC)

1. **Ajustes del Sistema → General → Compartir**.
2. Activa **Pantalla remota** (*Screen Sharing*).
3. En el detalle (icono ⓘ), activa **Los visores VNC pueden controlar la
   pantalla con contraseña** y define una contraseña VNC. Añade los usuarios
   que puedan conectarse si usas "Sólo estos usuarios".
4. Deja **Inicio de sesión remoto** desactivado salvo que quieras SSH.

### 3. Energía: que el Mac no se duerma

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

### 4. FileVault

Si FileVault está activo, **tras un reinicio no hay acceso remoto** hasta
desbloquear el disco en la pantalla (Tailscale tampoco arranca). Para
reinicios planificados usa:

```shell
sudo fdesetup authrestart
```

que reinicia guardando la autenticación para desbloquear el disco solo. Si
el Mac está a la espera de desbloqueo, sólo queda un reinicio local.

### 5. RustDesk (opcional)

1. Instala RustDesk desde <https://rustdesk.com> (arrastra la app a
   Aplicaciones).
2. **Ajustes del Sistema → Privacidad y seguridad**: concede **Grabación de
   pantalla** y **Accesibilidad** a RustDesk; añade también **Monitoreo de
   entrada** si macOS lo pide.
3. Configura una contraseña permanente y, si quieres, restringe el acceso a
   una sola vez. El **ID de RustDesk no se guarda en el repo**: se copia a
   mano a la ThinkPad.

## ThinkPad (lado flake)

1. Reconstruye y arranca Tailscale:

   ```shell
   sudo nixos-rebuild switch --flake ~/nixdots#thinkpad
   sudo tailscale up
   ```

   (`nixdots.remoteAccess.enable = true` ya trae `services.tailscale` y los
   paquetes `remmina` y `rustdesk-flutter`.)

2. En Remmina, **Nueva conexión**:
   - **Protocolo**: VNC.
   - **Servidor**: el nombre MagicDNS del Mac (`mac-mini.<tailnet>.ts.net`)
     o su IP `100.x.y.z` según `tailscale status`.
   - **Contraseña**: la contraseña VNC del Mac.
   Guarda el perfil; a partir de ahí conectar es un doble clic.

3. RustDesk (opcional): abre RustDesk, pega el ID del Mac y su contraseña.

## Verificación rápida

```shell
tailscale status                    # ambos equipos en la misma tailnet
ping mac-mini.<tailnet>.ts.net      # resuelve por MagicDNS
remmina                             # doble clic en el perfil VNC
```
