# Third-Party Assets & Attribution Notice

This repository contains data files, protocol mappings, and game assets necessary for Minecraft protocol compatibility across multiple versions.

---

### Protocol Version Translation (ViaVersion / ViaBackwards / ViaRewind)
* **Files**: `assets/viabackwards/`, `assets/viarewind/`
* **Copyright**: © ViaVersion contributors (https://github.com/ViaVersion).
* **License**: GPLv3 / MIT.

---

### Minecraft Data
* **Files**: `assets/builtin_registries/`, `assets/tracked_data/`, `assets/meta_data_type/`.
* **Copyright**: © Mojang Studios / Microsoft Corporation.
* **Terms**: These files are extracted or derived from Minecraft client and server releases. They are provided solely for compatibility, server emulation, and interoperability under the terms of the [Minecraft End User License Agreement (EULA)](https://www.minecraft.net/en-us/eula) and [Mojang Brand and Assets Guidelines](https://www.minecraft.net/en-us/usage-guidelines).
* **Note**: Vanilla data packs are not distributed in this repository; `build.rs` downloads the server jars from Mojang's official release servers and extracts the registries and tags it needs into `assets/datapacks/`. These files are **not** licensed under this plugin's license and remain the intellectual property of Mojang Studios.
