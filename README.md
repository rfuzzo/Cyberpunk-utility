# Cyberpunk-utility

Some utility tools for Cyberpunk 2077 modding.

## Tweak-Doxygen

A small rust utility to convert and strip tweak records (<https://github.com/CDPR-Modding-Documentation/Cyberpunk-Tweaks>) to a c# class hierarchy for use with doxygen: <https://cdpr-modding-documentation.github.io/Cyberpunk-Tweaks/>

```cmd
tweakdox <SOURCE> <OUT>
```

## Cyberpunk Mod Optimizer (CMOP)

> Work in progress

A small rust utility to sort a modlist topologically according to ordering rules, as wall as output warnigns and notes.

Rules spec taken from [mlox - the elder scrolls Mod Load Order eXpert](https://github.com/mlox/mlox).

### Usage

> Subject to change!

```cmd
Usage: cmop.exe [OPTIONS] [COMMAND]

Commands:
  list    Lists the current mod load order
  sort    Sorts the current mod load order according to specified rules
  verify  Verifies integrity of the specified rules
  help    Print this message or the help of the given subcommand(s)    

Options:
  -v, --verbose  Verbose output
  -h, --help     Print help
  -V, --version  Print version
```

#### list

```cmd
Lists the current mod load order

Usage: cmop.exe list [ROOT]

Arguments:
  [ROOT]  Root game folder ("Cyberpunk 2077"). Default is current working directory [default: ./]

Options:
  -h, --help  Print help
```

#### sort

```cmd
Sorts the current mod load order according to specified rules

Usage: cmop.exe sort [OPTIONS] [ROOT]

Arguments:
  [ROOT]  Root game folder ("Cyberpunk 2077"). Default is current working directory [default: ./]  

Options:
  -r, --rules <RULES>        Folder to read sorting rules from. Default is ./cmop [default: ./cmop]
  -d, --dry-run              Just print the suggested load order without sorting
  -m, --mod-list <MOD_LIST>  Read the input mods from a file instead of checking the root folder   
  -h, --help                 Print help
```

#### verify

```cmd
Verifies integrity of the specified rules

Usage: cmop.exe verify [OPTIONS]

Options:
  -r, --rules <RULES>  Folder to read sorting rules from. Default is ./cmop [default: ./cmop]
  -h, --help           Print help
```
