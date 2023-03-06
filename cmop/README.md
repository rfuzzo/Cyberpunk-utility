# Cyberpunk Load Order Optimizer (ClOptimizer)

> see: [https://github.com/mlox/mlox/blob/master/Documentation/Rule%20Guidelines.md]

## Sorting

### [Order]

```txt
[Order]
MAO_Base.esm
MAO_Music.esm
MAO_InteriorWeather.esm
MAO_Weather.esm
TR_Travels*.esp
abotSiltStridersTR*.esp
```

### [NearStart]

```txt
[NearStart]
Morrowind.esm
Tribunal.esm
Bloodmoon.esm
```

### [NearEnd]

```txt
[NearEnd]
Merged Lands.esp
Mashed Lists.esp
```

## Warnings

### [Note]

```txt
[Note]
  !!! The Merged Dialogs feature of TESTOOL is widely considered to be broken, it will cause some mods to stop working, and it is recommended you do not use it.
Merged_Dialogs.esp
```

### [Requires]

> The [Requires] rule specifies that when the dependant expression (expr-1) is true, that the consequent expression (expr-2) must be true.

```txt
[Requires] ; ( Ref: "AK_47 mod Readme.txt" )
AK_47 mod.esp
Aduls_Arsenal.esp
```

```txt
[Requires] ; ( Ref: "AlchemyStockpileHelper10.zip\readme.txt" )
AlchemyStockpileHelper10_Trib_BM_Sri.esp
[ANY  Sris_Alchemy_BM.esp
      EcoAdj(Sri+Ingredients).esp]
```

```txt
[Requires]
Advanced Alchemy - LDA patch.esp
[ALL  Advanced Alchemy.esp
      [ANY  Dwemer Alchemy Set (1.00 R3).esp
            Lexa's Dwemer Alchemy V2 (EN).esm]]
```

### [Conflict]

> The [Conflict] rule specifies that if any two of the following expressions are true, then we print out the given message indicating a conflict problem.

```txt
[Conflict]
  These plugins all contain merged levelled lists by default, TES3Merge is recommended for merging levelled lists.
Merged Objects.esp
multipatch.esp
Mashed Lists.esp
```

### [Patch]

> The [Patch] rule specifies a mutual dependency as in the case of a patch plugin that modifies some original plugin(s), or that glues two more plugins together to make them compatible. We use this rule to say two things:

```txt
[Patch] ; ( Ref: "ACG2 NOM Compatible.txt" )
  When using "Acheron's Camping Gear 2.esp" and NOM, you should use the patch "ACG2 NOM Compatible.esp" to make Acheron's fires and cauldron NOM compatible.
ACG2 NOM Compatible.esp
[ALL  Acheron's Camping Gear 2.esp
      NOM 2.13.esp]
```

## Rules Logic

### Comments

```txt
;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;
;; @Adjustable Magicka Regen [Glassboy]
```

### Wildcards - * and VER

```txt
[Order]
Westly_Presents_FCOT.esp
AAA <VER> Addon - Westly.ESP
TR_Travels*.esp
```

### [Any]

```txt
[Conflict]
  Don't run Nemon's "Vivec Interiorator" with any version of Szazmyrr3's "112 Vivec Replacement", rather use "112_Vivec_Replacement_Interiorator_Compatable_Fixed.esp" on its own as that version of Szazmyrr3's mod includes Nemon's "Vivec Interiorator".
[ANY  112_Vivec_Replacement_Interiorator_Compatable_Fixed.esp
      112_Vivec_Replacement_v_1.02.esp
      112_Vivec_Replacement_v_1.02_Vivec_Only.esp]
Nemon's_Vivec_Interiorator.esp
```

### [All]

```txt
[Requires]
Advanced Alchemy - LDA patch.esp
[ALL  Advanced Alchemy.esp
      [ANY  Dwemer Alchemy Set (1.00 R3).esp
            Lexa's Dwemer Alchemy V2 (EN).esm]]
```

```txt
[Conflict]
  Don't run Nemon's "Vivec Interiorator" with any version of Szazmyrr3's "112 Vivec Replacement", rather use "112_Vivec_Replacement_Interiorator_Compatable_Fixed.esp" on its own as that version of Szazmyrr3's mod includes Nemon's "Vivec Interiorator".
[ANY  112_Vivec_Replacement_Interiorator_Compatable_Fixed.esp
      112_Vivec_Replacement_v_1.02.esp
      112_Vivec_Replacement_v_1.02_Vivec_Only.esp]
Nemon's_Vivec_Interiorator.esp
```

### [NOT]

```txt
[Note] ; ( Ref: "Readme - Tealpanda's Alchemy Essentials.txt" )
      ! If you do not have Tribunal, Bloodmoon, and "Tamriel Rebuilt" installed you will NEED to use the folder "Optional - Ingredient Retexture" to use most of these mods.
[ALL  [ANY  Tealpanda's Alchemy Essentials.esp
            TAE - essence potions.esp
            TAE - ingredient trader.esp
            AE - TR essences and effects.esp]
      [NOT  [ANY  Tribunal.esm
                  Bloodmoon.esm
                  Tamriel_Data.esm]]]
```

### Nesting

```txt
[Note] ; ( Ref: "Readme - Tealpanda's Alchemy Essentials.txt" )
      ! If you do not have Tribunal, Bloodmoon, and "Tamriel Rebuilt" installed you will NEED to use the folder "Optional - Ingredient Retexture" to use most of these mods.
[ALL  [ANY  Tealpanda's Alchemy Essentials.esp
            TAE - essence potions.esp
            TAE - ingredient trader.esp
            AE - TR essences and effects.esp]
      [NOT  [ANY  Tribunal.esm
                  Bloodmoon.esm
                  Tamriel_Data.esm]]]
```
