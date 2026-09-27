# mcmodgetter

A simple lightweight command line tool for Minecraft using the Modrinth API to automatically download mods into a specified folder, programmed in Rust.

This tool is primarily designed to be used by server admins to have an easier time downloading mods for their servers, for instance when they update to a new game version. If you are looking for a way to manage mods for your Minecraft client, you'll probably want something far more sophisticated, e.g. [Prism Launcher](https://prismlauncher.org).

**Currently, only Modrinth mods are supported, but Curseforge and Hangar support in the future are not out of the question.**

## How to use

mcmodgetter is a command line tool, intended to be executed via shell/terminal with some user arguments in order to proceed.

You can run the executable with the `--help` argument for a full documentation on all available commands and arguments.

A basic example of a successfully executing command would look like this:

`mcmodgetter.exe download -id P7dR8mSH -mcv 1.21.11`

This would download Fabric API from modrinth for Minecraft 1.21.11 and the Fabric mod loader to a 'mods' directory local to the executable*.

*\*on Microsoft Windows*
## Getting a mod's ID

1. Navigate to the mod on Modrinth (e.g. https://modrinth.com/mod/fabric-api)
2. Click on the three dots in the top right corner and click "Copy ID"

This will copy the mod's ID to your clipboard.

## Setting up a mod list file

mcmodgetter supports downloading multiple mods concurrently from a single command through the use of a plaintext file of mod IDs. Setting up such a file is very simple:

1. Create a text file in the same directory as the executable.
2. Paste the mod IDs for every mod you wish to download into the file line by line.
    * To "comment out" a line, use a `#` symbol as the first character in the line.
3. Run the executable with the `download -file <modlist file>` configuration with the name of the file you just created.

## Additional features

* ### Automatic dependency handling
    Dependencies are queried from Modrinth and downloaded automatically alongside your mods. This behavior can be disabled with the `--skipdeps` option.
* ### Mod file tracking
    Downloaded mods are tracked automatically in a hidden `.tracker.mcmg` file in the mod folder. This allows for old files to be automatically discarded when updating mods.
    
    This file is not meant to be edited manually. Do not edit it unless you're sure you know what you're doing.
* ### Other functions
    Besides downloading mods, mcmodgetter has a couple other abilities:
    * `readmods` reads a modlist file and queries Modrinth for project titles and descriptions, in case you forgot what mods your IDs correspond to

    * `checkmods` compares specified parameters to mods currently present in the mod folder, listing how many can be updated

    * `clearmods` quickly removes all tracked files from the mod folder and clears the hidden `.tracker.mcmg` file

## Downloading/Building

For Microsoft Windows, an executable for the latest release is included in the [Releases](https://github.com/hwschieding/mcmodgetter/releases) tab on the github. Note that this executable is unsigned and Windows will probably get mad at you for trying to run it. If you're worried that I'm trying to give you malware, build it from source.

For other platforms,
1. I don't have the ability to extensively test the capabilities of mcmodgetter on some other platforms.
2. It would simply be tedious to include a compiled build for every permutaion of, for example, Linux distributions.

Luckily, Rust makes it trivial to compile and build the code for any platform it supports, and mcmodgetter is a very small tool. You will need the Rust toolchain manager [Rustup](https://rust-lang.org/tools/install/) and a linker. The complete installation process is described in detail [here](https://doc.rust-lang.org/book/ch01-01-installation.html).

Once you have the necessary tools, you just need to clone the repo and run `cargo build --release` in the base directory to compile and build mcmodgetter. Failing that, you'll have to rely on whatever the compiler is telling you to proceed.

## Custom builds and redistributions [![GitHub License](https://img.shields.io/github/license/hwschieding/mcmodgetter?style=flat-square&logo=gnu&label=License)](LICENSE)

Refer to the [license](LICENSE). If your fork/custom build contains significant code changes, please keep in mind that it is not accossiated with or endorsed by me or mcmodgetter. I ask in this instance that you follow through with the basic courtesy of **changing the APP_USER_AGENT variable in [lib.rs](src/lib.rs)** to a unique user agent that makes it clear your project is not affiliated with me or mcmodgetter.

