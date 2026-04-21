# sex-forge

The official SexOS userspace scaffolding CLI.

## Usage

### Create a new application

```sh
sex-forge new <name> [--base <base-template>]
```

This will create a new application crate in `apps/<name>`.

Available base templates:
- `cosmic-files`
- `cosmic-panel`
- `cosmic-applet-network`
- `cosmic-settings`
- `cosmic-applets`
- `servo`
- `rust-media`

If no base is provided, a default "hello world" style application is created.

### Port an application

```sh
sex-forge port-from <source> <recipe>
```

This command is a stub and does not yet perform a real port.

### Upgrade a component

```sh
sex-forge upgrade <component>
```

Currently, only `terminal` is supported.
This command is a stub and does not yet perform a real upgrade.
