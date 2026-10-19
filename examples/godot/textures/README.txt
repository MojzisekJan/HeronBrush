================================================================================
  SEM PATRI TEXTURY
================================================================================

Tenhle adresar je JEDINE misto, kde textury lezi. Ctou z nej oba programy:

  Heron Brush   protoze GameConfig.cfg rika  "materials": { "root": "textures" }
                a game path je nastavena na NADRAZENOU slozku, tedy na koren
                Godot projektu, ne na tuhle slozku samotnou

  Godot         protoze scena odkazuje na res://textures/...

Nekopiruje se nic sem a tam. Jeden soubor, dva programy.


JMENO SOUBORU = JMENO TEXTURY
--------------------------------------------------------------------------------

    textures/cihly.png          v editoru je to textura "cihly"
    textures/kov/plech.png      v editoru je to textura "kov/plech"

Podslozky fungujou. V editoru je musis pro danou mapu zapnout v panelu
Materials, jinak jejich textury neuvidis v prohlizeci.

Podporovane pripony: png, jpg, jpeg, bmp, tga, webp.


VELIKOST OBRAZKU MA VYZNAM
--------------------------------------------------------------------------------

Z rozmeru v pixelech se pocitaji UV souradnice. Textura 64x64 pokryje 64
jednotek mapy pri meritku 1. Kdyz obrazek vymenis za jinak velky, textura se
na stenach zmensi nebo zvetsi. Editor i heron_brush ctou rozmer ze stejneho
souboru, takze to, co vidis v editoru, uvidis i ve hre.


PBR MAPY
--------------------------------------------------------------------------------

Staci je polozit vedle zakladniho obrazku se spravnou priponou v nazvu:

    textures/example.png              barva  (tuhle vidis v editoru)
    textures/example_normal.png       normalova mapa
    textures/example_roughness.png    drsnost

Rozpoznavane pripony nazvu: _normal, _roughness, _metallic, _emission,
_ao, _orm.

heron_brush z nich sam postavi StandardMaterial3D. V editoru se jako samostatne
textury neukazou, protoze GameConfig.cfg je vylucuje pres "excludes".

Alternativne muze mit kazda textura svou slozku:

    textures/example/example.png
    textures/example/example_normal.png

Funguje obojí, vyber si jedno a drz se ho.


KDYZ CHCES VLASTNI MATERIAL
--------------------------------------------------------------------------------

Nekdy nechces obycejny StandardMaterial3D, ale treba shader nebo material
s rucne nastavenym metallic. Zaloz ho v Godotu jako .tres do slozky
materials/ a pojmenuj ho stejne jako texturu:

    materials/door.tres     prebije   textures/door.png

Pak stav s prepinacem:

    heron_brush build maps/arena.map --game heron_brush --out res://maps --material-dir res://materials

Editor porad ukazuje textures/door.png jako nahled, Godot ale pouzije
tvuj material. Priklad takoveho souboru je v materials/door.tres.


SPECIALNI TEXTURY (nastroje)
--------------------------------------------------------------------------------

    clip.png      Plochu NEVIDIS, ale NARAZIS do ni. Na zjednodusovani kolizi.

    skip.png      Plocha se z meshe UPLNE VYHODI, brush ale zustane solidni.
                  Optimalizace: pet sten krychle das na skip, jedna zustane
                  videt, kolize je porad cela krychle.

    origin.png    Brush cely oblepeny timhle se nekresli ani nekoliduje,
                  jen urcuje otacecí bod entity (pant u func_door).

Do meshe se nikdy nedostanou.

Muzou byt i ve vlastni podslozce, treba textures/tools/clip.png. heron_brush je pozna
podle nazvu souboru, na slozce nezalezi.

Textura, po ktere by plocha zustala videt a prestala kolidovat, neexistuje.
Na to je entita func_illusionary.


CO SEM NEPATRI
--------------------------------------------------------------------------------

Soubory .import si sem generuje Godot sam. Nesahej na ne a nemaz je,
editor je ignoruje.
