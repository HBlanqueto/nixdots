# nixosTest de F0: arranca el daemon, consulta el bus y comprueba que un job
# con estado Running vuelve a Queued tras reiniciar el servicio.
{
  pkgs,
  inputs,
  daemon,
}:

pkgs.testers.nixosTest {
  name = "keel-userd-f0";

  nodes.machine =
    { ... }:
    {
      imports = [
        inputs.impermanence.nixosModules.impermanence
        ../../../core/userd.nix
      ];

      _module.args.inputs = inputs;

      keel.userd = {
        enable = true;
        package = daemon;
        persistence.enable = false;
      };

      system.stateVersion = "26.05";
    };

  testScript = ''
    bus = "io.github.HBlanqueto.Keel.Userd1"
    manager = "io.github.HBlanqueto.Keel.Userd1.Manager"
    job = "io.github.HBlanqueto.Keel.Userd1.Job"
    root = "/io/github/HBlanqueto/Keel/Userd1"

    machine.start()
    machine.wait_for_unit("keel-userd.service")
    machine.wait_until_succeeds(f"busctl --system status {bus}")

    backends = machine.succeed(
        f"busctl --system call {bus} {root} {manager} ListBackends"
    )
    assert "declared" in backends, backends
    assert "userdb" in backends, backends
    assert "true" in backends, backends

    machine.succeed("systemctl stop keel-userd.service")
    machine.succeed(
        "${daemon}/bin/keel-userd seed-job"
        " --state-dir /var/lib/keel-userd --user test --running"
    )
    machine.succeed("systemctl start keel-userd.service")
    machine.wait_for_unit("keel-userd.service")
    machine.wait_until_succeeds(f"busctl --system status {bus}")

    jobs = machine.succeed(f"busctl --system call {bus} {root} {manager} ListJobs")
    assert f"{root}/Job/1" in jobs, jobs

    state = machine.succeed(
        f"busctl --system get-property {bus} {root}/Job/1 {job} State"
    )
    assert "queued" in state, state
  '';
}
