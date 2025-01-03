# Syngestures: Linux Multi-Touch Protocol Userland Daemon

syngestures is a utility providing multi-gesture support for various Linux touchpad/trackpad drivers
implementing the [Linux Multi-Touch
Protocol](https://www.kernel.org/doc/Documentation/input/multi-touch-protocol.txt), such as
`xf86-input-synaptics`. [Read more about syngesture, the impetus for its development, and how it fits
into the X11/Wayland evdev/libinput ecosystem in the release
announcement](http://neosmart.net/blog/2020/multi-touch-gestures-on-linux/)

## Purpose and Design

syngestures is a daemon (background application) that listens for input events generated by your
touchpad or trackpad and detects when multi-touch gestures are performed. It can be configured
(globally or on a per-user level) to carry out user-defined actions when specific gestures are
recognized (with support for unique configurations per-device if you have multiple touchpads
installed).

It may be used alone or, more commonly, in conjunction with desktop environment/display server
integration/driver - we recommend using it with `xf86-input-synaptics` under X11 for the most
responsive and "natural" cursor movement and acceleration.

## Dependencies

Prebuilt syngesture binaries are statically compiled and have no runtime dependencies. Building
syngestures from source normally creates a dependency on libevdev, which should already be installed
if you're on any modern Linux distribution.

### Security Considerations

Depending on your system configuration, you may need to either add a udev rule or make sure your
user account is a member of a particular group in order to use `syngestures` without root
privileges. **It is not recommended to run syngestures as root or under `sudo` since syngestures
allows running arbitrary commands in response to touch gestures.**

Refer to the troubleshooting section below for more information.

## Installation

Packages containing prebuilt binaries for various systems are available for tagged syngestures
releases and can be obtained from GitHub.

syngestures is written in rust and requires a working copy of the rust toolchain and a functional C
compiler in order to build from source. It can be compiled and installed by checking out a copy of
the source code and building with `cargo`, the rust package manager:

```
git clone https://github.com/mqudsi/syngesture.git
cd syngesture
cargo install --path .
```

alternatively, it may be downloaded, built, and installed directly via cargo:

```
cargo install syngestures
```

## Configuration

syngesture is configured via one or more TOML configuration files, a sample file [is included in this
repository](./syngestures.toml). Configuration files may be installed at a machine level to
`/usr/local/etc/syngestures.toml` or with multiple per-device configuration files installed to
`/usr/local/etc/syngestures.d/*.toml`, or at a user level with a configuration file at
`$HOME/.config/syngestures.toml` or multiple per-device configuration files installed to
`$HOME/.config/syngestures.d/*.toml`. Multiple files are supported and concatenated with user
configuration files overriding the system configuration file.

The basic format of the configuration file is as follows, with a `[[device]]` node per input device
implementing the MT protocol:

```toml
[[device]]
device = "/dev/input/by-path/pci-0000:00:15.0-platform-i2c_designware.0-event-mouse"
gestures = [
	# Navigate next
	{ type = "swipe", direction = "right", fingers = 3, execute = "xdotool key alt+Right" },
	# Navigate previous
	{ type = "swipe", direction = "left", fingers = 3, execute = "xdotool key alt+Left" },
	# Next desktop/workspace
	{ type = "swipe", direction = "right", fingers = 4, execute = "xdotool key Super_L+Right" },
	# Previous desktop/workspace
	{ type = "swipe", direction = "left", fingers = 4, execute = "xdotool key Super_L+Left" },
]
```

The value of `device` should be a stable path to your touchpad, it can often be found by looking at
the output of `dmesg`. Wayland users may substitute the usage of `xdotool` for whatever alternative
supports their display server/compositor/window manager.

The value of each gesture's `type` may be either `swipe` or `tap`; a numeric `fingers` parameter
from `1` to `5` is required in both cases, but an additional `direction` (being one of `right`,
`left`, `up`, or `down`) is required in case of `swipe`.

## Troubleshooting

If you get an error like the following when using syngestures (the path to the device depends on the
path you've set up in `syngestures.toml`):

> /dev/input/by-path/pci-0000:00:15.0-platform-i2c_designware.0-event-mouse: Permission denied (os error 13)

then your account does not have sufficient privileges to open the input device and listen for
events. Typically working around this is as easy as adding your account to the `input` group but we
can find out for sure by figuring out which group owns the input device.

Start by using `ls -al` against the path of the device, e.g. from the error message above:

```sh
$ ls -al /dev/input/by-path/pci-0000:00:15.0-platform-i2c_designware.0-event-mouse
lrwxrwxrwx 1 root root 9 Feb 11 16:16 /dev/input/by-path/pci-0000:00:15.0-platform-i2c_designware.0-event-mouse -> ../event4
```

As you can see from the output, in this case our `/dev/input/...` device is actually a symlink to
another character device, so we need to use `ls -al` again to find out who actually owns it,
replacing the final part of the device path (`pci-0000..`) with the symlink target (`../event4`):

```sh
$ ls -al /dev/input/by-path/../event4
crw-rw---- 1 root input 13, 68 Feb 11 16:16 /dev/input/by-path/../event4
```

Here the third column is the owning user (`root`) and the fourth column is the owning group
(`input`). We need to add our account to this group to give us permission to open the input
device and listen for gestures, replacing `input` in the code below with the owning group you
identified on your system if it differs:

```sh
$ sudo usermod -aG input $(whoami)
```

**You must log out completely then log back in (or just reboot) before this change will take
effect.** You can then try running `syngestures` again and see what happens.

## License

syngestures is developed and maintained by Mahmoud Al-Qudsi and released as open source under the
MIT license, copyright NeoSmart Technologies 2020-2023.
