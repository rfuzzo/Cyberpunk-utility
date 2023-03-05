# Cyberpunk-utility

Some utility tools for Cyberpunk 2077 modding.

## Tweak-Doxygen

A small rust utility to convert and strip tweak records (<https://github.com/CDPR-Modding-Documentation/Cyberpunk-Tweaks>) to a c# class hierarchy for use with doxygen: <https://cdpr-modding-documentation.github.io/Cyberpunk-Tweaks/>

### Usage

```cmd
tweakdox <SOURCE> <OUT>
```

## Cyberpunk Loadorder Optimizer (ClOptimizer)

> Work in progress

A small rust utility to sort a modlist topologically according to ordering rules, as wall as output warnigns and notes.

Rules spec taken from [mlox - the elder scrolls Mod Load Order eXpert](https://github.com/mlox/mlox).

### Usage

> Subject to change!

```cmd
cl_optimizer "./modlist.txt" "./rules_base.txt"
```
