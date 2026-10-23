# nixosTest de F0: arranca el daemon, consulta el bus y comprueba que un job
# con estado Running vuelve a Queued tras reiniciar el servicio.
{
  pkgs,
  inputs,
  daemon,
}:

pkgs.testers.nixosTest {
  name = "nixos-userd-f0";

  nodes.machine =
    { ... }:
    {
      imports = [
        inputs.impermanence.nixosModules.impermanence
        ../../../core/userd.nix
      ];

      _module.args.inputs = inputs;

      services.nixos-userd = {
        enable = true;
        package = daemon;
        persistence.enable = false;
      };

      system.stateVersion = "26.05";
    };

  testScript = ''
    bus = "io.github.HBlanqueto.Userd1"
    manager = "io.github.HBlanqueto.Userd1.Manager"
    job = "io.github.HBlanqueto.Userd1.Job"
    root = "/io/github/HBlanqueto/Userd1"

    machine.start()
    machine.wait_for_unit("nixos-userd.service")
    machine.wait_until_succeeds(f"busctl --system status {bus}")

    backends = machine.succeed(
        f"busctl --system call {bus} {root} {manager} ListBackends"
    )
    assert "declared" in backends, backends
    assert "userdb" in backends, backends
    assert "true" in backends, backends

    machine.succeed("systemctl stop nixos-userd.service")
    machine.succeed(
        "${daemon}/bin/nixos-userd seed-job"
        " --state-dir /var/lib/nixos-userd --user test --running"
    )
    machine.succeed("systemctl start nixos-userd.service")
    machine.wait_for_unit("nixos-userd.service")
    machine.wait_until_succeeds(f"busctl --system status {bus}")

    jobs = machine.succeed(f"busctl --system call {bus} {root} {manager} ListJobs")
    assert f"{root}/Job/1" in jobs, jobs

    state = machine.succeed(
        f"busctl --system get-property {bus} {root}/Job/1 {job} State"
    )
    assert "queued" in state, state
  '';
}
