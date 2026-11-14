# Third-Party Assets & Attribution Notice

This repository contains data files, protocol mappings, and game assets necessary for Minecraft protocol compatibility across multiple versions.

---

### Protocol Version Translation (ViaVersion / ViaBackwards / ViaRewind)
* **Files**: `assets/viabackwards/`, `assets/viarewind/`
* **Copyright**: © ViaVersion contributors (https://github.com/ViaVersion).
* **License**: GPLv3 / MIT.

---

### Minecraft Data
* **Files**: `assets/tracked_data/`, `assets/meta_data_type/`.
* **Copyright**: © Mojang Studios / Microsoft Corporation.
* **Terms**: These files are extracted or derived from Minecraft client and server releases. They are provided solely for compatibility, server emulation, and interoperability under the terms of the [Minecraft End User License Agreement (EULA)](https://www.minecraft.net/en-us/eula) and [Mojang Brand and Assets Guidelines](https://www.minecraft.net/en-us/usage-guidelines).
* **Note**: Vanilla data is not distributed in this repository. `build.rs` downloads the server jars from Mojang's official release servers and writes what it needs into the git-ignored `assets/datapacks/` at build time: the jar's data files for 1.19.4 and newer, and the reports of the jar's own data generator for the registries built into the game before 1.19.4. These files are **not** licensed under this plugin's license and remain the intellectual property of Mojang Studios.
