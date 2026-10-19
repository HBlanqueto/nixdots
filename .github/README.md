# Keel

Dotfiles personales para NixOS, montados como un único flake.

> [!WARNING]
> Esto **no es una distribución de Linux**. Son mis dotfiles personales, muy
> hardcodeados a mi hardware y a mi forma de trabajar. No están recomendados
> como sistema principal y cambian sin aviso; un cambio puede romperlo todo.

## Instalación

```shell
nix-shell -p git --run "git clone --depth 1 https://github.com/HBlanqueto/nixdots"
sudo ./nixdots/install.sh
```

- [Instalador, Secure Boot y cifrado](docs/installer.md)
- [Sistema, herramientas y acceso remoto](docs/system.md)

## Credits

Special thanks to these individuals whose work greatly inspired and guided the configuration of these files:

* [SergioRibera's dotfiles](https://github.com/SergioRibera/dotfiles)
* [chxp82q's nix-config](https://github.com/chxp82q/nix-config)
