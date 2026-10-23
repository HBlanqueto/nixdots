# Decisiones de nixos-userd

Registro de las decisiones que no se explican solas en el código: lo que se
cerró al implementar F0 y lo que queda pendiente para fases futuras.

## F0 (fundaciones)

- **Nombres del bus.** Todas las constantes viven en
  `kits/userd-core/src/names.rs` (`BUS_NAME`, `ROOT_PATH`,
  `ERROR_NAMESPACE`, `POLKIT_PREFIX` e interfaces). Es el prefijo
  `io.github.HBlanqueto.Userd1` en F0; pasar a `org.nixos.*` es cambiar esas
  constantes. Los XML de `system/services/userd/interfaces/` y el derive
  `DbusError` necesitan literales por limitación de zbus; el test de
  conformidad (`bus.rs`) falla si dejan de coincidir con las constantes.
- **Superficie D-Bus de F0.** El XML de `Manager` solo declara lo implementado
  (`ListJobs`, `ListBackends`, `ConfigRejected`) y el de `Job` las propiedades,
  `Cancel` y `Finished`. El resto de interfaces del plan llega con su fase:
  escribir métodos sin implementación habría roto el test de conformidad.
- **Esquema KDL.** `version` es obligatorio y solo se acepta `1`; el resto de
  nodos y propiedades se rechaza si no se conoce, para que una errata no pase
  como campo ignorado. El "KDL v1" del plan es la versión del esquema: la
  sintaxis es KDL v2 (crate `kdl` 6). Los valores de `home.option` deben ser
  escalares (bool, número, string o null); arrays y objetos se rechazan.
- **Nombre de archivo.** `<usuario>.kdl` debe coincidir con el campo `user` del
  documento; si no, el watcher lo rechaza.
- **Jobs en F0.** El scheduler (actor por usuario con `mpsc` y semáforo global)
  está completo, pero no hay handlers registrados: un job encolado se queda en
  `Queued` en vez de inventar una ejecución. `Running` y `Blocked` vuelven a
  `Queued` al arrancar; la re-evaluación de `Blocked` la hará el handler en F1
  (vuelve a bloquear si la condición sigue).
- **Job de prueba.** `nixos-userd seed-job` es el gancho mínimo para que el
  nixosTest pueda comprobar la reanudación sin `CreateUser`. Es un comando
  interno, no D-Bus, y se elimina cuando F1 cree jobs de verdad. Usa
  `JobKind::ModifyUser` como marcador.
- **Store.** Escritura atómica (temporal en el mismo directorio, fsync del
  archivo, rename y fsync del directorio) con permisos 0600. El watcher inotify
  mantiene en memoria el último estado válido: si un KDL editado a mano no
  parsea, no reemplaza nada y se emite `Manager.ConfigRejected`. En el arranque
  los `ConfigRejected` se encolan y se emiten cuando el bus ya está conectado.
- **ListBackends.** Sondea los seis backends conocidos en el orden de prioridad.
  `userdb` se informa no disponible si `/etc/userdb` no se puede escribir
  (`access(W_OK)`); el daemon no crea ni asume escribible `/etc`. `homed` y
  `flatpak` miran el bus (propietario o activable); `nix-profile` el socket del
  daemon; `home-manager` la presencia de `inputs.json` (que F2 generará).
  `declared` siempre está disponible.
- **Módulo NixOS.** `services.nixos-userd.enable = false` por defecto: ningún
  equipo cambia en F0. Con `enable = true`, el módulo añade `stateDir` a
  `environment.persistence."/persist".directories` (si `persistence.enable`,
  que por defecto es `true`) y crea la config, el servicio, la política D-Bus y
  la de polkit. F0 no aplica polkit: las acciones se declaran ahora para que F1
  solo tenga que comprobarlas.
- **Sin tocar crewd.** `system/services/crewd`, `home-init`,
  `system/users/default.nix` y `system/lib/data.nix` siguen siendo la fuente de
  verdad hasta F1.

## Fases futuras (no implementado)

- **F1 — crewd.** nixos-userd sustituye a crewd: retirar el servicio, los TOML
  de `/persist/keel/users`, `hashedPasswordFile` y su aserción en
  `system/core/user.nix`, y dar un camino de migración para las cuentas
  existentes de la ThinkPad (leer los TOML y crear los usuarios con userdb o
  homed antes de apagar lo viejo).
- **F1 — contraseñas con userdb.** El hash va en `<name>.user-privileged`
  (0600), nunca en `/etc/shadow` (con `mutableUsers = false` se pierde). Con
  homed, en el registro de homed. `SetPassword` nunca pasa por `pam_unix`.
- **F1 — admin = wheel.** `wheel` vive en `/etc/group`; el usuario de
  userdb/homed entra por membresía de userdb. Comprobar pronto que sudo y polkit
  lo reconocen; si no, documentar la alternativa antes de seguir.
- **F1 — persistencia.** Añadir a `environment.persistence."/persist"` también
  `/var/lib/systemd/home` y `/etc/userdb`.
- **Primer arranque.** Hoy `firstBoot` sale de `system/lib/data.nix` al evaluar.
  Pasará a decidirse en runtime (`HasNoUsers`), y el asistente de `welcome/`
  llamará a `CreateUser` en vez de escribir TOML y reconstruir.
- **F2 — home.** Pendiente decidir cómo convive con el flake del usuario en
  `~/.config/home-manager` (plantillas deck/raft y steward-home): los dos
  escriben el mismo perfil. No implementar F2 sin resolverlo.
- **F6 — capacidades.** Las unidades de NixOS no tienen `[Install]`, así que
  nada de `EnableUnitFiles`: usar `StartUnit`/`StopUnit` y reaplicar al arrancar
  el estado guardado en redb.
- **Build del home.** Preferir `nix build --impure --expr` sobre un flake
  generado (evita gestionar `flake.lock`).
