# Testing on a Windows VM

A Windows 11 guest in VirtualBox, on another workstation of the maintainer's
(the `om` ssh alias), driven from the notebook over SSH: commands, file copies,
screenshots and key presses, with no agent inside the guest. [Wine](wine.md) is
the quicker check on Linux; this is the real one.

## Reaching the guest

The guest sits behind VirtualBox's NAT (guest address `10.0.2.15`). A NAT port
forward bound to the host's loopback carries SSH, so nothing listens on the
host's network ports:

```sh
# on the host, once (survives guest reboots; stored in the VM's settings)
VBoxManage controlvm Win11 natpf1 "ssh,tcp,127.0.0.1,2222,,22"

# from the notebook
ssh -J om -p 2222 topaxi@127.0.0.1 'whoami'
scp -o ProxyJump=om -P 2222 target/x86_64-pc-windows-gnu/release/oag-game.exe topaxi@127.0.0.1:
```

The guest's default SSH shell is Windows PowerShell 5.1.

## Guest setup (once, by hand, elevated)

1. Install the OpenSSH Server capability (`Add-WindowsCapability -Online -Name OpenSSH.Server~~~~0.0.1.0`), start `sshd`, set it to Automatic.
2. Put the notebook's public key in `C:\ProgramData\ssh\administrators_authorized_keys` (the account is an administrator), ACL limited to Administrators and SYSTEM (`icacls <file> /inheritance:r /grant "Administrators:F" /grant "SYSTEM:F"`).
3. In `C:\ProgramData\ssh\sshd_config`: `PasswordAuthentication no` and `KbdInteractiveAuthentication no`; `Restart-Service sshd`.
4. The firewall rule `OpenSSH-Server-In-TCP` must cover the **Public** profile: the NAT adapter's network is Public, and the rule the capability creates covers Private only, so a connection through the forward times out with no banner. `Set-NetFirewallRule -Name OpenSSH-Server-In-TCP -Profile Any`. A `Test-NetConnection 10.0.2.15 -Port 22` from inside the guest passes either way (it skips the inbound firewall), so test through the forward.
5. Optional: PowerShell as the default shell (`HKLM:\SOFTWARE\OpenSSH`, `DefaultShell`).

## What runs over SSH, and what does not

A process started over SSH runs in a session with no desktop: fine for
`--dump-audio`, logs, file listings and anything headless; a window or a GPU
present does not appear. For a window, start it in the logged-in user's session
(a scheduled task with `/IT`, `schtasks /create ... /it` then `/run`), and drive
it from the host:

```sh
ssh om 'VBoxManage controlvm Win11 screenshotpng /tmp/win11.png'   # what the desktop shows
ssh om 'VBoxManage controlvm Win11 keyboardputstring "y"'          # type into the focused window
ssh om 'VBoxManage controlvm Win11 keyboardputscancode 1c 9c'      # Enter
```

Disc images reach the guest by `scp` like any file; keep them on the VM, never
committed or uploaded anywhere. The game finds one in `%APPDATA%\oag\images\`.
