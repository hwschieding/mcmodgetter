# MCModGetter

A simple lightweight command line tool for Minecraft using the Modrinth API to automatically download mods into a specified folder, made in Rust.

This tool is primarily designed to be used by server admins to have an easier time downloading mods for their servers, for instance when they update to a new game version. If you are looking for a way to manage mods for your Minecraft client, you'll probably want something far more sophisticated.

**Currently, only Modrinth mods are supported, but Curseforge and Hangar support in the future are not out of the question.**

## How to use

MCModGetter is a command line tool, and will have to be executed via terminal with some user arguments in order to proceed.

You can run the executable with the `--help` argument for a full documentation on all available commands and arguments.

A basic example of a successfully executing command would look like this:

`mcmodgetter.exe download -id P7dR8mSH -mcv 1.21.11`

This would download Fabric API from modrinth for Minecraft 1.21.11 and the Fabric mod loader to a 'mods' directory local to the executable.

## Getting a mod's ID

1. Navigate to the mod on Modrinth (e.g. https://modrinth.com/mod/fabric-api)
2. Click on the three dots in the top right corner and click "Copy ID"

This will copy the mod's ID to your clipboard.

## Setting up a mod list file

MCModgetter supports downloading multiple mods concurrently from a single command through the use of a plaintext file of mod IDs. Setting up such a file is very simple:

1. Create a text file in the same directory as the executable.
2. Paste the mod IDs for every mod you wish to download into the file line by line.
    * To "comment out" a line, use a `#` symbol as the first character in the line.
3. Run the executable with the option `--readfile <modlist file>` with the name of the file you just created.