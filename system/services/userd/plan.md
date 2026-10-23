> **nombre definitivo: keel-userd**

# nixos-userd — Plan de desarrollo

Daemon para NixOS que gestiona usuarios, configuración del home, paquetes y capacidades del sistema **en runtime**, sin modificar la configuración del sistema y sin `nixos-rebuild`. Lo consumen apps de configuración y tiendas (propias, de GNOME o de KDE) a través de D-Bus.

> Convenciones de código: seguir la skill `rust-engineering` (edition 2024, `clippy::pedantic` limpio, errores `thiserror` sin `anyhow`, sin `unwrap`/`expect` en producción, `foo.rs` + `foo/` en lugar de `mod.rs`, `tracing`, código y comentarios en inglés). Las decisiones de diseño que surjan durante la implementación se registran en `CLAUDE.md`, no en comentarios.

---

## 1. Objetivos y no-objetivos

### Objetivos
- Crear, modificar y eliminar usuarios en runtime.
- Gestionar la configuración del home (vía home-manager) en runtime, con generaciones y rollback.
- Instalar y desinstalar paquetes por usuario sin rebuild (`nix profile`, home-manager, Flatpak).
- Activar y desactivar capacidades del sistema que estén pre-declaradas en NixOS.
- Ser compatible con los estándares D-Bus existentes (`org.freedesktop.Accounts`, `org.freedesktop.PackageKit`) para que GNOME Settings, GDM, KDE, Discover y GNOME Software funcionen sin cambios.
- Definir un protocolo D-Bus propio y estándar (`org.nixos.Userd1.*`) para lo que freedesktop no cubre, pensado para proponerse upstream a nixpkgs.
- Ser usable por cualquier distro o shell basada en NixOS. Primeros consumidores: dos shells Wayland en desarrollo.

### No-objetivos (por ahora)
- Historial de cambios de la configuración.
- Snapshots de datos del home.
- Cambios de sistema que requieran nueva evaluación de NixOS (drivers, módulos del kernel, servicios no pre-declarados). Se reportan como "requiere actualización del sistema".

---

## 2. Decisiones cerradas

| Tema | Decisión |
|---|---|
| Nombre | `nixos-userd` |
| Bus name | `org.nixos.Userd1` |
| Object path raíz | `/org/nixos/Userd1` |
| Fuente de verdad | Archivos KDL por usuario |
| Parser KDL | crate `kdl` v6 (spec KDL v2), mapeo propio a tipos del core vía `TryFrom` |
| Identidad | `homed` y `userdb`, elegidos por disponibilidad y prioridad; más `declared` (usuarios de `users.users`, solo lectura) |
| Paquetes | `nix-profile`, `home-manager`, `flatpak`, elegidos por disponibilidad y prioridad |
| Compatibilidad | `org.freedesktop.Accounts` completa (reemplaza `accounts-daemon`) y backend PackageKit |
| Runtime async | `tokio` |
| D-Bus | `zbus` (feature `tokio`) y `zbus_polkit` |
| Persistencia de jobs | `redb` |
| Historial / snapshots | Fuera del alcance actual |

---

## 3. Arquitectura

```
┌────────────────┐  ┌──────────────┐  ┌───────────────┐  ┌─────────────┐
│ App de config  │  │ Tienda propia│  │ GNOME Settings│  │ Discover /  │
│ (shell propia) │  │              │  │ GDM / KDE     │  │ GNOME Softw.│
└───────┬────────┘  └──────┬───────┘  └───────┬───────┘  └──────┬──────┘
        │ org.nixos.Userd1.*                  │ org.freedesktop │ org.freedesktop
        │                                     │ .Accounts       │ .PackageKit
        ▼                                     ▼                 ▼
┌─────────────────────────────────────────────────────┐  ┌──────────────┐
│                  nixos-userd (root)                 │◄─┤ packagekitd +│
│  bus · authz(polkit) · jobs · registry              │  │ userd backend│
│  identity: homed | userdb | declared                │  └──────────────┘
│  home: home-manager runtime                         │
│  packages: nix-profile | home-manager | flatpak     │
│  capabilities: systemd1                             │
└───┬───────────┬──────────────┬──────────────┬───────┘
    ▼           ▼              ▼              ▼
 home1/     nix-daemon     flatpak        systemd1
 userdb     + store        helper
```

Principios:
- **El daemon es el único escritor.** Ninguna app toca archivos, todo pasa por D-Bus y polkit.
- **Las operaciones largas son jobs.** Las llamadas D-Bus nunca bloquean: devuelven un objeto `Job` y emiten progreso.
- **Un actor por usuario.** Cada usuario tiene una cola (`tokio::sync::mpsc`) que serializa sus operaciones. Un `tokio::sync::Semaphore` global limita los builds concurrentes de Nix.
- **El backend se elige al crear o instalar** y queda registrado. Las operaciones posteriores van siempre a ese backend.

---

## 4. Workspace

```
nixos-userd/
├── Cargo.toml                  # workspace, edition 2024
├── CLAUDE.md                   # decisiones de diseño
├── crates/
│   ├── userd-core/
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── ids.rs          # Username, Uid, Gid, JobId
│   │       ├── spec.rs         # UserSpec, HomeSpec, PackageRef, Capability
│   │       ├── spec/
│   │       │   ├── identity.rs
│   │       │   ├── home.rs
│   │       │   ├── packages.rs
│   │       │   └── accounts.rs # overlay cosmético (icono, idioma, sesión)
│   │       ├── kdl.rs          # TryFrom<&KdlDocument> / Into<KdlDocument>
│   │       ├── job.rs          # JobState, BlockReason, JobKind, JobRecord
│   │       ├── backend.rs      # trait Backend, Availability, BackendKind, prioridades
│   │       └── error.rs
│   ├── userd-daemon/
│   │   └── src/
│   │       ├── main.rs
│   │       ├── config.rs       # config del daemon generada por el módulo NixOS
│   │       ├── bus.rs          # registro de interfaces zbus
│   │       ├── bus/
│   │       │   ├── manager.rs
│   │       │   ├── home.rs
│   │       │   ├── packages.rs
│   │       │   ├── capabilities.rs
│   │       │   ├── job.rs
│   │       │   └── accounts.rs # org.freedesktop.Accounts (+ User)
│   │       ├── authz.rs        # polkit
│   │       ├── jobs.rs         # scheduler, journal redb, reanudación por eventos
│   │       ├── registry.rs     # UIDs, tombstones, índice de usuarios
│   │       ├── store.rs        # lectura/escritura atómica de KDL, watcher inotify
│   │       ├── identity.rs     # trait IdentityBackend, selección
│   │       ├── identity/
│   │       │   ├── homed.rs
│   │       │   ├── userdb.rs
│   │       │   └── declared.rs
│   │       ├── home.rs
│   │       ├── home/
│   │       │   ├── flake.rs    # generación del flake por usuario
│   │       │   └── activate.rs # build + activación como el usuario
│   │       ├── packages.rs     # trait PackageBackend, búsqueda unificada
│   │       ├── packages/
│   │       │   ├── nix_profile.rs
│   │       │   ├── home_manager.rs
│   │       │   └── flatpak.rs
│   │       ├── capabilities.rs
│   │       └── runner.rs       # ejecución de comandos como otro usuario (systemd-run)
│   └── userd-packagekit/       # cdylib, backend de PackageKit
├── interfaces/                 # XML de introspección (fuente del spec)
│   ├── org.nixos.Userd1.Manager.xml
│   ├── org.nixos.Userd1.Home.xml
│   ├── org.nixos.Userd1.Packages.xml
│   ├── org.nixos.Userd1.Capabilities.xml
│   └── org.nixos.Userd1.Job.xml
├── polkit/
│   └── org.nixos.userd.policy
└── nix/
    ├── flake.nix
    ├── module.nix              # services.nixos-userd
    ├── hm-module.nix           # módulo home-manager que consume el JSON
    └── tests/                  # nixosTest por fase
```

### Dependencias principales
- `tokio`, `zbus` (feature `tokio`), `zbus_polkit`
- `kdl` (v6), `serde`, `serde_json` (JSON User Records y JSON para HM)
- `redb` (journal de jobs y registro)
- `thiserror`, `tracing`, `tracing-subscriber`, `tracing-journald`
- `notify` (inotify sobre el directorio de usuarios)
- `rustix` (consultas de usuarios y permisos, sin `unsafe` propio)

---

## 5. Modelo de datos

### 5.1 Archivos

| Ruta | Contenido | Permisos |
|---|---|---|
| `/var/lib/nixos-userd/users/<name>.kdl` | Fuente de verdad del usuario | root `0600` |
| `/var/lib/nixos-userd/state.redb` | Jobs, registro de UIDs, índice | root `0600` |
| `/var/lib/nixos-userd/hm/<name>/` | Flake generado, `config.json`, GC roots | root `0700` |
| `/var/lib/nixos-userd/archive/<name>-<ts>/` | Homes archivados al borrar | root `0700` |
| `/etc/userdb/<name>.user` | JSON User Record (backend userdb) | generado |
| `/etc/nixos-userd/config.json` | Config del daemon, generada por el módulo NixOS | read-only |
| `/etc/nixos-userd/inputs.json` | Store paths fijados de nixpkgs y home-manager | read-only |
| `/var/lib/AccountsService/users/<name>` | Datos de extensiones de Accounts (compat) | root |

Sin secretos en KDL: las contraseñas se gestionan vía homed (`org.freedesktop.home1`) o PAM.

### 5.2 Esquema KDL v1 (borrador)

```kdl
version 1

user "s4rch" {
    identity {
        backend "homed"          // registrado al crear; no cambia solo
        uid 60001
        real-name "Sergio"
        shell "/run/current-system/sw/bin/nu"
        groups "video" "audio"
        admin #false             // privilegiado: solo admin puede cambiarlo
    }

    accounts {                   // overlay cosmético compatible con Accounts
        icon "/var/lib/AccountsService/icons/s4rch"
        language "es_BO.UTF-8"
        session "niri"
        automatic-login #false
    }

    home {
        option "programs.git.enable" #true
        option "programs.git.userName" "Sergio"
    }

    packages {
        package "ripgrep" backend="nix-profile" attr="nixpkgs#ripgrep"
        package "helix" backend="home-manager" attr="helix"
        package "org.mozilla.firefox" backend="flatpak" remote="flathub"
    }
}
```

Reglas:
- `version` es obligatorio y permite migraciones del esquema.
- Los campos se clasifican como **privilegiados** (`uid`, `groups`, `admin`, `shell`, `backend`), que requieren `users.manage`, o **del usuario** (resto), que basta con `*-own`.
- Escritura atómica: archivo temporal, `fsync` y `rename`.
- Edición manual por root: el watcher la detecta, la valida y, si es inválida, la rechaza conservando el último estado válido en memoria. Se emite `Manager.ConfigRejected`.

### 5.3 Tipos del core (forma, no implementación)

```rust
pub struct Username(String);   // validado: [a-z_][a-z0-9_-]{0,31}
pub struct Uid(u32);
pub struct JobId(u64);

pub enum IdentityBackendKind { Homed, Userdb, Declared }
pub enum PackageBackendKind { NixProfile, HomeManager, Flatpak }

pub enum Availability { Available, Unavailable(UnavailableReason) }

pub enum JobState {
    Queued,
    Running { progress: Option<Percent> },
    Blocked(BlockReason),
    Succeeded,
    Failed(JobFailure),
    Cancelled,
}

pub enum BlockReason {
    HomeLocked,
    UserSessionUnavailable,
    BackendUnavailable(BackendKind),
    NetworkUnavailable,
    AwaitingAuth,
}

pub enum JobKind {
    CreateUser, ModifyUser, ArchiveUser,
    ApplyHome, RollbackHome,
    InstallPackage, RemovePackage,
    EnableCapability, DisableCapability,
}
```

Para los usuarios declarados se usa `UserSpec` con `IdentityBackendKind::Declared`: la identidad es de solo lectura y solo se aceptan cambios en `accounts`, `home` y `packages`.

---

## 6. Selección de backends por disponibilidad

```rust
pub trait Backend {
    fn kind(&self) -> BackendKind;
    async fn probe(&self) -> Availability;
}
```

- Prioridades configurables desde NixOS, por ejemplo `identity.priority = [ "homed" "userdb" ]` y `packages.priority = [ "nix-profile" "flatpak" "home-manager" ]`.
- Al crear un usuario o instalar un paquete sin backend explícito, se usa el primero disponible según la prioridad. El backend elegido se escribe en el KDL.
- Las operaciones posteriores usan siempre el backend registrado. Si no está disponible, el job pasa a `Blocked(BackendUnavailable)` y se reanuda cuando el backend vuelve.
- Disponibilidad cacheada y re-evaluada con `NameOwnerChanged` y con los cambios de las unidades relevantes.

| Backend | Probe |
|---|---|
| homed | `org.freedesktop.home1` en `ListActivatableNames` o con owner |
| userdb | `systemd` en `passwd:` de `/etc/nsswitch.conf` y `/etc/userdb` escribible |
| declared | Siempre disponible (lee NSS / `/etc/passwd`) |
| nix-profile | Socket de `nix-daemon` accesible |
| home-manager | `inputs.json` presente y nix-profile disponible |
| flatpak | Binario `flatpak` y `org.freedesktop.Flatpak.SystemHelper` |

---

## 7. Modelo de jobs

- Toda operación con efectos crea un job persistido en `redb` **antes** de ejecutarse.
- Al iniciar, el daemon recarga los jobs no terminales: `Running` vuelve a `Queued` (las operaciones deben ser idempotentes) y `Blocked` re-evalúa su condición.
- Reanudación por eventos:
  - `HomeLocked`: cambio de estado en `org.freedesktop.home1` (unlock / activate)
  - `UserSessionUnavailable`: aparición de `user@<uid>.service`
  - `BackendUnavailable`: `NameOwnerChanged` y re-probe
  - `NetworkUnavailable`: `network-online.target`
- Cancelación: `Job.Cancel()` solo en `Queued` o `Blocked`. Un job `Running` se cancela si su backend lo soporta.
- Retención: los jobs terminales se purgan pasado un tiempo configurable.
- Cada job se expone como `/org/nixos/Userd1/Job/<id>` mientras exista.

---

## 8. Protocolo D-Bus

Las firmas usan la notación D-Bus. Los specs XML en `interfaces/` son la fuente de verdad, y los servidores `zbus` deben coincidir con ellos (un test lo verifica).

### 8.1 `org.nixos.Userd1.Manager` en `/org/nixos/Userd1`

Métodos:
- `ListUsers() → ao`
- `FindUserByName(s name) → o`
- `CreateUser(s name, a{sv} options) → o job` — `options`: `real-name`, `backend`, `admin`, `groups`, `shell`
- `ArchiveUser(o user, b keep_home) → o job`
- `ListBackends() → a(ssb)` — `(kind, name, available)`
- `ListJobs() → ao`

Señales:
- `UserAdded(o)`, `UserRemoved(o)`
- `BackendAvailabilityChanged(s kind, s name, b available)`
- `ConfigRejected(s name, s reason)`

### 8.2 `org.nixos.Userd1.User` en `/org/nixos/Userd1/User/<uid>`

Propiedades: `Name s`, `Uid u`, `Backend s`, `Managed b` (`false` para declarados), `State s` (`active` | `locked` | `archived`).

### 8.3 `org.nixos.Userd1.Home` en el objeto del usuario

- `GetConfig() → a{sv}`
- `SetOption(s path, v value) → ()` — valida contra la lista blanca, no aplica
- `UnsetOption(s path) → ()`
- `Apply() → o job`
- `ListGenerations() → a(uts)` — `(id, timestamp, store_path)`
- `Rollback(u generation) → o job`
- Propiedad `Dirty b`: hay cambios sin aplicar.

### 8.4 `org.nixos.Userd1.Packages` en el objeto del usuario

- `Search(s query, as backends) → a(sssss)` — `(backend, id, name, version, summary)`
- `ListInstalled() → a(sss)` — `(backend, id, version)`
- `Install(s id, s backend) → o job` — `backend` vacío aplica las prioridades
- `Remove(s id) → o job` — usa el backend registrado

### 8.5 `org.nixos.Userd1.Capabilities` en `/org/nixos/Userd1`

- `List() → a(ssb)` — `(name, description, enabled)`
- `Enable(s name) → o job`
- `Disable(s name) → o job`
- Señal `CapabilityChanged(s name, b enabled)`

### 8.6 `org.nixos.Userd1.Job` en `/org/nixos/Userd1/Job/<id>`

- Propiedades: `Kind s`, `Target o`, `State s`, `BlockReason s`, `Progress u` (0–100, o `u32::MAX` si es indeterminado), `Error s`
- `Cancel() → ()`
- Señal `Finished(s state)`

### 8.7 Errores
Namespace `org.nixos.Userd1.Error.*`: `NotFound`, `PermissionDenied`, `InvalidConfig`, `BackendUnavailable`, `ReadOnly` (identidad de usuario declarado), `Conflict`, `Failed`.

### 8.8 polkit (`polkit/org.nixos.userd.policy`)

| Acción | Uso | Default activo |
|---|---|---|
| `org.nixos.userd.users.manage` | Crear, archivar y campos privilegiados | `auth_admin_keep` |
| `org.nixos.userd.home.modify-own` | Home propio | `yes` |
| `org.nixos.userd.home.modify-any` | Home ajeno | `auth_admin_keep` |
| `org.nixos.userd.packages.install-own` | Paquetes propios | `yes` |
| `org.nixos.userd.packages.install-any` | Paquetes de otro usuario | `auth_admin_keep` |
| `org.nixos.userd.capabilities.toggle` | Capacidades | `auth_admin_keep` |

Las acciones "own" se resuelven comparando el UID del caller (credenciales D-Bus) con el `Uid` del objeto destino.

---

## 9. Backends en detalle

### 9.1 Identidad

**homed**
- `CreateHome` / `UpdateHome` / `RemoveHome` vía `org.freedesktop.home1`, con el JSON User Record generado desde el KDL.
- El rango de UIDs lo asigna homed (60001–60513). Se registra en el registry.
- Respetar `storage` configurable desde NixOS (`luks`, `fscrypt`, `directory`, `subvolume`).

**userdb**
- Escribe `/etc/userdb/<name>.user`, más los symlinks por UID (`<uid>.user`) y los registros de grupo que requiere la spec de userdb.
- Rango de UIDs propio y configurable. Los asigna el registry y nunca se reutilizan (tombstones).
- Crea el home con `systemd-tmpfiles` o directamente, con los permisos correctos.

**declared**
- Enumera los usuarios de `users.users` (el módulo NixOS exporta la lista en `config.json`).
- Identidad de solo lectura: cualquier cambio de identidad devuelve `ReadOnly`.
- Permite overlay en `accounts`, `home` y `packages`.

### 9.2 Ejecución como otro usuario (`runner.rs`)
- Se evita `unsafe` (`pre_exec`, `setgroups`) usando `systemd-run`:
  - Comandos sin sesión: `systemd-run --uid=<uid> --gid=<gid> --pipe --wait --collect`
  - Comandos que necesitan la sesión del usuario (Flatpak `--user`, activación de HM): `systemd-run --machine=<user>@.host --user --pipe --wait --collect`
- Sin `user@<uid>.service` activo, el job pasa a `Blocked(UserSessionUnavailable)`. Opción NixOS para habilitar linger por defecto.
- El tipo `Runner` encapsula esto con errores tipados (`SpawnFailed`, `ExitStatus { code, stderr }`).

### 9.3 Home runtime (home-manager)
1. Traducir `home` del KDL a `hm/<name>/config.json`.
2. Generar `hm/<name>/flake.nix`, que importa `inputs.json` (mismos nixpkgs y home-manager que el sistema) y el módulo `hm-module.nix`, el cual lee el JSON con `builtins.fromJSON`.
3. Build como root vía nix-daemon (`nix build --out-link hm/<name>/generation-<n>`). El out-link es el GC root.
4. Activación como el usuario con `runner` ejecutando `<generation>/activate`.
5. Con homed y el home cerrado: `Blocked(HomeLocked)`.

Lista blanca de opciones: el módulo NixOS define `services.nixos-userd.home.allowedOptions` (lista de paths). `SetOption` rechaza lo que no esté ahí con `InvalidConfig`. Escape hatch para usuarios avanzados: opción `services.nixos-userd.home.allowExtraNix` que permite un `extra.nix` por usuario (solo admin).

### 9.4 Paquetes
- **nix-profile:** `nix profile install|remove --profile <home>/.local/state/nix/profiles/profile`, ejecutado vía `runner` como el usuario. `Search` usa un índice pre-generado de nixpkgs (de `inputs.json`) en lugar de `nix search` en vivo, que es lento.
- **home-manager:** agrega o quita `packages` en el KDL y encadena un `ApplyHome`.
- **flatpak:** `flatpak --user install|uninstall` vía `runner` en la sesión del usuario. Desinstalar puede borrar datos de la app, así que la UI debe advertirlo (`Remove` admite la opción `keep-data`).

### 9.5 Capacidades
- El módulo NixOS declara `services.nixos-userd.capabilities.<name> = { description; units = [ ... ]; }`. Esas unidades se instalan sin `wantedBy`.
- `Enable` / `Disable` vía `org.freedesktop.systemd1`: `EnableUnitFiles` con `runtime=true` (escribe en `/run`, porque `/etc` es inmutable en NixOS) y `StartUnit`/`StopUnit`.
- El estado deseado se persiste en `redb`, y al boot el daemon re-aplica las capacidades habilitadas.

---

## 10. Compatibilidad `org.freedesktop.Accounts`

El daemon toma el bus name `org.freedesktop.Accounts` y el módulo NixOS deshabilita `accounts-daemon`.

Superficie a implementar completa:
- `org.freedesktop.Accounts`: `ListCachedUsers`, `FindUserById`, `FindUserByName`, `CreateUser`, `CacheUser`, `UncacheUser`, `DeleteUser`, `GetUsersLanguages`, señales `UserAdded` / `UserDeleted` y propiedades `DaemonVersion`, `HasNoUsers`, `HasMultipleUsers`, `AutomaticLoginUsers`.
- `org.freedesktop.Accounts.User`: todos los setters (`SetUserName`, `SetRealName`, `SetEmail`, `SetLanguage`, `SetFormatsLocale`, `SetXSession`, `SetSession`, `SetSessionType`, `SetLocation`, `SetHomeDirectory`, `SetShell`, `SetIconFile`, `SetLocked`, `SetAccountType`, `SetPasswordMode`, `SetPassword`, `SetPasswordHint`, `SetAutomaticLogin`), todas las propiedades (`Uid`, `UserName`, `RealName`, `AccountType`, `HomeDirectory`, `Shell`, `IconFile`, `Locked`, `PasswordMode`, `LoginFrequency`, `LoginTime`, `LoginHistory`, `SystemAccount`, `LocalAccount`, `Language`, `Session`, `SessionType`, `XSession`, …) y la señal `Changed`.
- **Mecanismo de extensiones:** leer las interfaces XML en `share/accountsservice/interfaces/` y servirlas en cada objeto usuario, persistiendo en `/var/lib/AccountsService/users/<name>` con el mismo formato keyfile que accountsservice.

Mapeo:
- `CreateUser` y `DeleteUser` se traducen a jobs de `Userd1.Manager`. La llamada Accounts espera a que el job termine (con timeout) para mantener la semántica síncrona.
- `SetPassword` se redirige a homed o PAM.
- `AccountType` admin equivale a pertenecer a `wheel` (campo privilegiado).
- Usuarios declarados: los setters cosméticos escriben el overlay `accounts`, y los de identidad devuelven `org.freedesktop.Accounts.Error.PermissionDenied`.
- `SystemAccount` / `LocalAccount`: replicar exactamente la heurística de accountsservice (rangos de UID, shells válidas), porque GDM depende de ellas.

Verificación: portar la test suite de accountsservice como oráculo y correr VMs con GDM y SDDM.

---

## 11. Backend PackageKit (`userd-packagekit`)

- `cdylib` que implementa la interfaz de backend de PackageKit con C ABI. Es el único crate con `unsafe`, cada bloque con `// SAFETY:`.
- Traduce `Search*`, `Resolve`, `GetDetails`, `InstallPackages`, `RemovePackages`, `GetPackages` y `GetUpdates` a llamadas a `org.nixos.Userd1.Packages` del usuario que origina la transacción, y mapea el progreso de los jobs a las señales de la transacción.
- El ABI de backends no es estable entre versiones de PackageKit: la versión se fija en el paquete Nix y hay un test de carga del backend en CI.
- El módulo NixOS configura `services.packagekit` con `DefaultBackend=userd`.

---

## 12. Módulo NixOS (`services.nixos-userd`)

```nix
services.nixos-userd = {
  enable = true;
  identity = {
    priority = [ "homed" "userdb" ];
    homed.storage = "luks";
    userdb.uidRange = { from = 30000; to = 59999; };
  };
  packages.priority = [ "nix-profile" "flatpak" "home-manager" ];
  home = {
    allowedOptions = [ "programs.git.*" "programs.helix.*" ];
    allowExtraNix = false;
  };
  capabilities.printing = {
    description = "Impresión (CUPS)";
    units = [ "cups.service" "cups.socket" ];
  };
  accountsCompat.enable = true;   # deshabilita accounts-daemon
  packagekitCompat.enable = true;
  lingerByDefault = true;
  jobs.retention = "7d";
};
```

El módulo:
- Habilita `systemd.services.nixos-userd`, la policy de polkit y la config D-Bus.
- Habilita `services.homed` y `nss-systemd` según las prioridades.
- Genera `/etc/nixos-userd/config.json` (incluye la lista de usuarios declarados) e `inputs.json`.
- Expone `nixos-userd.homeModules.fromUserd` para el flake por usuario.

---

## 13. Fases

Cada fase termina con: `cargo fmt`, `cargo clippy --all-targets -- -W clippy::pedantic` limpio, `cargo test` y su `nixosTest` pasando.

### F0 — Fundaciones
- Workspace, CI (fmt, clippy pedantic, tests, `nix flake check`).
- `userd-core`: ids, specs, esquema KDL v1 con `TryFrom` en ambos sentidos y tests de round-trip.
- Trait `Backend`, `Availability` y resolución por prioridad.
- Modelo de jobs: tipos, journal `redb`, scheduler con actor por usuario y reanudación tras reinicio.
- Interfaz `org.nixos.Userd1.Job` y `Manager.ListJobs` / `ListBackends`.
- `store.rs`: escritura atómica y watcher con rechazo de configs inválidas.
- Esqueleto del módulo NixOS y un `nixosTest` que arranca el daemon y consulta el bus.
- `CLAUDE.md` con las decisiones de esta sección.

**Aceptación:** el daemon arranca en una VM, lista backends con su disponibilidad y un job de prueba sobrevive a un reinicio del servicio.

### F1 — Identidad
- Backends homed, userdb y declared.
- `Manager.CreateUser`, `FindUserByName`, `ArchiveUser`, objetos `User`.
- Registry de UIDs con tombstones.
- polkit (`authz.rs`) para todas las acciones de usuario.
- Archivado del home al borrar.

**Aceptación (VM):** crear usuario con homed, apagar homed y verificar que el siguiente usuario cae en userdb; login de ambos; borrar uno y confirmar que su UID no se reasigna; intentar modificar la identidad de un usuario declarado devuelve `ReadOnly`.

### F2 — Home runtime
- `runner.rs` con `systemd-run`.
- Generación del flake, build, GC roots y activación como el usuario.
- `Home.SetOption`, `Apply`, `ListGenerations`, `Rollback` y lista blanca.
- Bloqueos `HomeLocked` / `UserSessionUnavailable` con reanudación.

**Aceptación (VM):** activar `programs.git` para un usuario sin rebuild; rollback a la generación anterior; con homed cerrado el job queda `Blocked` y termina al hacer login.

### F3 — Paquetes
- Backends nix-profile, home-manager y flatpak.
- Índice de búsqueda pre-generado y `Search` unificado.
- `Install` / `Remove` con backend registrado en el KDL.

**Aceptación (VM):** instalar un paquete por cada backend, removerlos, y verificar que un `Remove` con el backend caído queda `Blocked`.

### F4 — Compatibilidad total con Accounts
- Interfaces `Accounts` y `Accounts.User` completas, extensiones y keyfiles.
- Heurísticas de `SystemAccount` / `LocalAccount` idénticas a accountsservice.
- Test suite de accountsservice portada.

**Aceptación (VM):** GDM lista los usuarios y permite login; GNOME Settings y KDE crean, editan y borran usuarios; los usuarios declarados aceptan cambios cosméticos y rechazan los de identidad.

### F5 — Backend PackageKit
- `cdylib`, mapeo de transacciones y progreso.

**Aceptación (VM):** `pkcon search/install/remove` funciona, y GNOME Software instala y desinstala.

### F6 — Capacidades
- Opción del módulo, `Capabilities.*` vía `systemd1` con overrides en `/run` y re-aplicación al boot.

**Aceptación (VM):** habilitar impresión sin rebuild, reiniciar la VM y que siga habilitada.

### F7 — Settings de sistema y spec
- Glue para `locale1`, `timedate1` y `hostname1` (el módulo cede `time.timeZone = null`, etc.).
- Documentación del spec D-Bus desde los XML y RFC para nixpkgs.

**Mínimo para las shells propias:** F0–F3. F4 desbloquea GDM, GNOME Settings y KDE.

---

## 14. Estrategia de pruebas

- **Unitarias (`userd-core`):** parseo y round-trip KDL, validación de `Username`, transiciones de `JobState`, resolución por prioridad.
- **Integración (`userd-daemon`):** backends con fakes detrás de los traits; bus de sesión privado con `zbus` para probar las interfaces sin root.
- **Conformidad del spec:** test que compara la introspección de los servidores `zbus` con los XML de `interfaces/`.
- **`nixosTest`:** una VM por fase con los criterios de aceptación; VMs adicionales con GDM y SDDM para F4.
- **Resiliencia:** matar el daemon a mitad de un job y verificar que se reanuda; editar a mano un KDL inválido y verificar el rechazo.

---

## 15. Caveats y riesgos

- **homed bloquea el home sin sesión** (LUKS / fscrypt): las operaciones sobre el home quedan en `Blocked(HomeLocked)`. `storage = "directory"` lo evita, a costa del cifrado.
- **homed y SSH:** requiere `AuthorizedKeysCommand` en `sshd`. Documentar y configurar desde el módulo.
- **Compatibilidad Accounts:** GDM depende de detalles finos (`SystemAccount`, `LocalAccount`, caché de iconos, historial de login). Es la fase con más riesgo y la test suite de accountsservice es el oráculo.
- **ABI de PackageKit inestable:** versión fijada y test de carga en CI.
- **Primer build de HM sin cache:** puede tardar minutos. Pre-calentar el binary cache con el módulo base y mostrar progreso indeterminado.
- **Store legible por todos:** lo que termina en `/nix/store` (config de HM generada) es visible para otros usuarios. No poner datos sensibles en opciones de HM.
- **Idempotencia:** como los jobs `Running` se re-encolan tras un reinicio, cada operación debe ser segura de repetir.
- **Flatpak:** desinstalar puede borrar datos; exponer `keep-data`.
- **Usuarios declarados:** el daemon nunca escribe su identidad; si `users.mutableUsers = false`, cualquier escritura en `/etc/passwd` se pierde en la siguiente activación, por eso solo se usan userdb y homed.
- **Conflicto de bus names:** `accounts-daemon` y `nixos-userd` no pueden coexistir con `accountsCompat.enable = true`; el módulo lo garantiza con una aserción.

---

## 16. Preguntas abiertas

- Formato y generación del índice de búsqueda de nixpkgs (¿derivación en el sistema o descargado del cache?).
- Granularidad de la lista blanca de opciones de HM (¿globs por path o derivada de los metadatos de opciones de home-manager?).
- Política de retención de generaciones de HM por usuario.
- Si la búsqueda unificada debe deduplicar un mismo programa disponible en varios backends.
