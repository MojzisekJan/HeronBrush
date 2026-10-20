# Začínáme
Heron Brush je editor úrovní založený na brushích. Úrovně skládáte z konvexních brushů, jejich stěny texturujete a barvíte, umisťujete entity, animujete dveře a plošiny a výsledek exportujete do herního enginu. K dispozici jsou také vestavěné assety, parametrické tvary, živé propojení s editory enginů, společné úpravy a vykreslování statických obrázků. Tato příručka je součástí aplikace a funguje bez připojení k internetu.

Na macOS funguje `Cmd` všude, kde příručka uvádí `Ctrl`, a Option odpovídá klávese Alt.

## Uvítací okno
Když Heron Brush spustíte bez mapy, otevře se uvítací okno. Vlevo jsou tlačítka **Pokračovat v** (naposledy otevřená mapa), **Nová mapa** a **Procházet** pro otevření existující `.map`, dále **Hra**, **Příručka** a **Nastavení**. Vpravo je seznam nedávných map s náhledy a pod ním **Příklady**. Hvězdička (**Připnout nahoru**) připne mapu na začátek seznamu, koš (**Přesunout do koše**) přesune soubor do systémového koše, odkud ho lze obnovit. Šipkami se pohybujete po okně, Enter otevře zvolenou položku. Kdykoli se k oknu vrátíte přes Soubor → Vítejte.

## První mapa
1. Na uvítací obrazovce zvolte **Nová mapa**, nebo Soubor → Nová mapa (`Ctrl+N`).
2. Na liště nástrojů vyberte nástroj Brush (`B`) a tažením ve 2D pohledu nakreslete zvolený tvar. Hloubku zkontrolujte v jiném 2D pohledu.
3. Brush vyberte a tažením přesuňte. Na kartě **Stěna** mu přiřaďte materiál nebo barvu.
4. Zdrojovou mapu uložte pomocí `Ctrl+S`. Až ji budete chtít použít ve hře, otevřete Soubor → Export a zapište scénu pro svůj engine.

## Soubory a ukládání
Soubor `.map` je váš upravitelný zdroj. Ukládejte průběžně: export, živé propojení ani vykreslování mapu za vás neuloží. Soubor → Uložit jako (`Ctrl+Shift+S`) doplní příponu `.map`, pokud ji vynecháte. Soubor → Vrátit se k uložené mapě zahodí neuložené změny a načte soubor znovu. Pokud zavíráte okno nebo otevíráte jinou mapu a máte neuložené změny, Heron Brush se zeptá, zda **Uložit**, **Zahodit** nebo **Zrušit**.

Pokud má uložená mapa neuložené změny, zapisují se každých deset minut číslované zálohy do složky `autosave` vedle mapy. Uchovává se 50 nejnovějších záloh.

Otevřete-li mapu, když už Heron Brush běží, předá se běžícímu oknu a to se přesune do popředí. Chcete-li samostatné okno, použijte na příkazové řádce `--new-instance` (viz Příkazová řádka).

# Rozhraní
Hlavní okno má titulkový pruh s nabídkami, pod ním lištu nástrojů, mapové pohledy, inspektor vpravo a stavový řádek dole.

## Titulkový pruh a nabídky
Titulkový pruh obsahuje logo, nabídky, uprostřed název mapy a tlačítka okna. Na macOS jsou obvyklá červená, žlutá a zelená tlačítka vlevo. Tažením za pruh okno přesunete, dvojklikem ho maximalizujete. Když něco běží na pozadí, objeví se u konce pruhu indikátor: živé propojení s enginem, živá relace nebo **MCP aktivní**. Kliknutím na indikátor živé propojení zastavíte nebo otevřete příslušné okno.

- **Soubor**: Vítejte, Nová mapa (`Ctrl+N`), Otevřít mapu (`Ctrl+O`), Uložit (`Ctrl+S`), Uložit jako (`Ctrl+Shift+S`), Vrátit se k uložené mapě, Export, Vykreslit obrázek, Pluginy a Nastavení (`Ctrl+,`).
- **Úpravy**: Zpět (`Ctrl+Z`), Znovu (`Ctrl+Shift+Z`), Vyjmout, Kopírovat, Vložit, Duplikovat (`Ctrl+D`), Smazat, Vybrat vše (`Ctrl+A`), Opakovat poslední příkazy (`Ctrl+R`) a Zapomenout opakovatelné příkazy.
- **Zobrazení**: 3D pohled v samostatném okně, Přejít na, Přichytávat k mřížce (`Alt+0`), Zobrazit mřížku (`0`), Náhled animací a Zobrazit vynechané stěny.
- **Geometrie**: Vložit 3D text, Spojit roh, Převést stěny na pláty, Vyjmout výběr z jeho entity a Sladit provázané skupiny.
- **Animace**: Animace klíčovými snímky, Náhled animací a Umístit kameru podle 3D pohledu (`Ctrl+Alt+0`).
- **Mapa**: načtení, opětovné načtení a zavření bodového souboru a souboru portálů. Ukazují stopy netěsností a portály, které zapisují kompilátory map.
- **Spolupráce**: Živá relace a Zpřístupnit agentovi (MCP).
- **Nápověda**: Příručka a O programu.

## Lišta nástrojů

V Nastavení → Zobrazení → Okno zapněte **Zobrazit tlačítko náhledu simulací**. Vedle Lock a UV v horní liště se objeví tlačítko pro spuštění a zastavení náhledu. Sdílí stav s příkazem Animace → Náhled animací. Volba se zachová po restartu; ve výchozím nastavení je vypnutá.
Lišta nástrojů nese všechny nástroje. První skupinu používáte neustále: Vybrat, Brush, Řez, Vrchol, Hrana, Stěna, Otočit a Měřítko. Ve druhé skupině jsou Zkosit, Skládat, Protažení, Řídicí bod, Zaoblení a Plechovka. Po najetí myší na tlačítko se zobrazí název a zkratka. Opětovné kliknutí na aktivní nástroj vrátí nástroj Vybrat.

Za nástroji následují **Překlopit vodorovně** a **Překlopit svisle** a dva přepínače: **Zámek** zachová mapování materiálu při posunu brushe a **UV** při jeho otočení (přepíná ho `U`). Pak přijdou nastavení aktivního nástroje: nástroj Brush ukazuje tvar a jeho volby, nástroj Zaoblení poloměr, počet ploch, rohy a tlačítko **Použít**. Na pravém konci je název nástroje, velikost mřížky (**Mřížka 16** apod.; kliknutím zvolíte jinou) a nabídka **Nastavení mřížky** s položkami Přichytávat k mřížce, Zobrazit mřížku, Náhled animací, Zobrazit vynechané stěny a Animace klíčovými snímky.

## Mapové pohledy
K dispozici jsou čtyři pohledy: 3D pohled a tři ploché pohledy Shora (XY), Zepředu (XZ) a Zboku (YZ). Nastavení → Zobrazení → Rozvržení nabízí Jeden panel, Dva panely, Tři panely nebo Čtyři panely. Tažením přepážek měníte velikost panelů. Kliknutím do pohledu mu předáte klávesnici. `Space` přepne panel, který ukazuje jeden pohled po druhém, na další pohled, a `Ctrl+Space` maximalizuje pohled s klávesnicí a znovu obnoví rozvržení.

Kliknutím pravým tlačítkem bez tažení v libovolném pohledu otevřete místní nabídku pro výběr a to, co je pod ukazatelem. Nabízí seskupování, vrstvy, provázané skupiny, vytváření entit, přesun brushů do entity a z ní, kopírování názvu materiálu a jeho ukázání v prohlížeči materiálů.

## Inspektor
Inspektor vpravo má čtyři karty. Šířku změníte tažením jeho levého okraje.

- **Mapa** (`Ctrl+1`): Vrstvy, Skupiny, Pluginy, Problémy, filtry Zobrazení a Značky.
- **Entita** (`Ctrl+2`): prohlížeč entit, definice entit, vlastnosti vybrané entity a u tlačítek a spouštěčů Spouštění animace.
- **Stěna** (`Ctrl+3`): UV, Materiály, Barva, Protažení, Zaoblení, Plát a Zapuštění a zkosení. Každou sekci lze sbalit.
- **Assety**: knihovna znovupoužitelných modelů z brushů.

`Ctrl+5` skryje nebo zobrazí inspektor a `Ctrl+Alt+T` skryje nebo zobrazí lištu nástrojů, takže pohledy získají více místa.

## Stavový řádek a výstupní panel
Stavový řádek dole ukazuje poslední zprávu editoru. Chyby jsou červeně. Kliknutím na řádek nebo klávesami `Ctrl+4` otevřete výstupní panel s celou historií zpráv. Většina příkazů sem hlásí, co udělala, například kolik brushů rozřízla.

## Dialogy a vzhled
Dialogy jako Nastavení, Export a Vykreslit obrázek se otevírají ve vlastních oknech. Nastavení → Zobrazení → Barevné schéma nabízí Podle systému, Světlé nebo Tmavé. **Krytí panelů (%)** na stránce Přehled zprůhlední panely nad pohledy. Okno kreslí vlastní tlačítka ve stylu systému: nativní na macOS, ve stylu Windows 11 na Windows a ve stylu GNOME na Linuxu.

# Ovládání a klávesnice
## Pohyb ve 3D pohledu
Držte pravé tlačítko myši a tažením se rozhlížejte. Tažením prostředním tlačítkem posouváte pohled. Kolečko pohybuje dopředu a dozadu. `W`, `A`, `S` a `D` letí dopředu, doleva, dozadu a doprava, `Q` stoupá a `X` klesá; se `Shift` jsou kroky větší. Je-li v Nastavení → Myš zapnuto **Alt + tažení prostředním tlačítkem hýbe kamerou**, Alt s prostředním tlačítkem posouvá kameru ve směru pohledu. Rychlosti, převrácení os a zorný úhel najdete v Nastavení → Myš.

## Pohyb ve 2D pohledech
Tažením prostředním tlačítkem posouváte pohled, kolečko přibližuje kolem ukazatele. Posun a přiblížení jednoho plochého pohledu udrží ostatní zarovnané; to řídí **Provázat 2D pohledy** v Nastavení → Myš. `Ctrl+Alt+Z` obnoví přiblížení a `Ctrl+Shift+U` vrátí ploché pohledy na počátek.

`Ctrl+U` zarámuje výběr ve všech pohledech. Zobrazení → Přejít na (`Ctrl+Alt+M`) přesune kameru na zadané souřadnice, vybere objekt na daném řádku souboru mapy nebo posune výběr o zadanou vzdálenost.

## Mřížka a přichytávání
`+` a `-` mění velikost mřížky a číselné klávesy `1` až `9` volí velikost přímo, od 1 do 256 jednotek. `0` nebo `#` zobrazí či skryje mřížku a `Alt+0` zapne nebo vypne přichytávání. `S` přichytí vrcholy vybraných brushů k aktuální mřížce, `Ctrl+Shift+V` k celým jednotkám a `Ctrl+Alt+Shift+V` k mřížce.

## Běžné zkratky
- `Ctrl+S`: uložit mapu. `Ctrl+Shift+S`: uložit jako.
- `Ctrl+Z`: zpět. `Ctrl+Shift+Z` nebo `Ctrl+Y`: znovu.
- `Ctrl+C`, `Ctrl+X`, `Ctrl+V`: kopírovat, vyjmout a vložit. Vložení umístí objekty k ukazateli; `Ctrl+Alt+V` zachová původní souřadnice. Jsou-li vybrané stěny, kopíruje se jejich materiál a nastavení UV.
- `Ctrl+D`: duplikovat. `Ctrl+Shift+D`: provázaná kopie skupiny.
- `Delete` nebo `Backspace`: smazat výběr.
- `Esc`: zrušit probíhající tažení, vrátit krok v aktivním nástroji, přepnout na Vybrat a nakonec zrušit výběr. `Shift+Esc` přepne rovnou na Vybrat.
- `Ctrl+R`: zopakovat poslední opakovatelné příkazy. `Ctrl+Shift+R`: zapomenout je.
- `Ctrl+Alt+0` nebo `Ctrl+Alt+Num0`: umístit renderovací kameru podle 3D pohledu.
- `F5`: znovu načíst materiály. `F6`: znovu načíst definice entit.

## Zkratky pro výběr
- `Ctrl+A`: vybrat vše, nebo všechny úchyty, je-li aktivní nástroj Vrchol, Hrana nebo Stěna. `Ctrl+Shift+A`: zrušit výběr. `Ctrl+Alt+A`: obrátit výběr.
- `Ctrl+B`: vybrat ostatní objekty ve stejné entitě nebo skupině.
- `Ctrl+T`: vybrat vše, čeho se vybrané brushe dotýkají. `Ctrl+E`: vybrat vše uvnitř nich. `Ctrl+Shift+E`: vybrat vše v jejich sloupci podél osy pohledu. Výběrové brushe se přitom spotřebují.
- `Ctrl+I`: izolovat výběr. `Ctrl+Alt+I`: skrýt ho. `Ctrl+Shift+I`: znovu zobrazit vše.
- `Ctrl+G`: seskupit. `Ctrl+Shift+G`: zrušit skupinu.

## Zkratky pro transformace
- Šipky a `Page Up` / `Page Down` posunou výběr o jeden krok mřížky ve směru, kterým ukazují v aktivním pohledu. S `Ctrl` při posunu duplikují, s `Alt` otáčejí.
- `Alt+X`, `Alt+Y`, `Alt+Z`: otočit o čtvrt otáčky kolem dané osy; se `Shift` opačným směrem.
- `Ctrl+F`: překlopit napříč pohledem; `Ctrl+Alt+F`: překlopit nahoru a dolů. `Ctrl+Shift+X` a `Ctrl+Shift+Y` zrcadlí podél X nebo Y.
- `Insert`: umístit bodovou entitu do středu výběru.
- `Tab`: s vybraným brushem přepíná Vybrat → Vrchol → Hrana → Stěna → Vybrat.

## Sada zkratek Blender
Nastavení → Klávesnice → Sada zkratek nabízí Heron Brush (výchozí, používá ji celá příručka) a Blender. Se sadou Blender: `G` vybírá a posouvá, `R` otáčí, `S` mění měřítko, `E` protahuje, `1`, `2` a `3` volí nástroje Vrchol, Hrana a Stěna, `K` řeže, `B` vybírá, `A` vybere vše a `Alt+A` výběr zruší, `X` maže, `Shift+D` duplikuje, `H` skryje a `Alt+H` vše zobrazí a `Ctrl+Shift+Z` provede znovu. Vše ostatní je v obou sadách stejné.

# Brushe a modelovací nástroje
Brushe jsou konvexní tělesa definovaná rovinami. Složité objekty skládejte z několika jednoduchých brushů, nesnažte se udělat jeden brush nekonvexní. Kde stačí jednoduchý tvar, držte počet brushů nízko.

## Kreslení brushů
Zvolte nástroj Brush (`B`) a táhněte v pohledu. Lišta nástrojů ukazuje **Kreslený tvar**: Kvádr, Schodiště, Oblouk (klenba), Válec, Kužel, Sféroid (UV), Sféroid (dvacetistěn), Dutá koule a Vložit 3D text. `Alt+1` až `Alt+8` volí tvar přímo. V okně **Nastavení tvaru** najdete **Dutý** a **Tloušťka:**, u schodů **Výška schodu:** a **Směr:**, u kulatých tvarů **Osa:**, **Stran:** a režim kružnice, u UV koulí **Prstenců:**, u ikosaedru **Přesnost:** a u duté koule **Brushe**. Volby nastavte před tažením. Výsledek zkontrolujte ve více pohledech.

### Dutá koule z brushů

Vyberte **Dutá koule**, případně stiskněte `Alt+8`. Nastavte **Tloušťka:** a
**Brushe** v okně **Nastavení tvaru**, pak tvar nakreslete jako ostatní
brushe. Počet lze volit od 20 do 320 po čtyřech, například 76, 80 či 84.
Protilehlé hrany se zjemňují společně, aby koule zůstala středově souměrná.
Ručně zadaný počet se zaokrouhlí na nejbližší podporovanou hodnotu. Hloubka kreslení
vychází z referenčních rozměrů stejně jako u ostatních tvarů. Stejné rozměry
ve všech třech osách vytvoří kouli, různé rozměry dutý elipsoid.

Každý díl má trojúhelníkovou vnější a vnitřní stěnu a tři kontaktní stěny.
Kontakty na sousedy přesně navazují a používají `skip`; střed zůstává prázdný.
Tloušťka se měří radiálně ve vrcholech na nejkratší poloose, musí být kladná
a menší než tato poloosa. U protaženého elipsoidu se v ostatních směrech
zvětší úměrně rozměrům. Výsledkem jsou běžné vybrané brushe s jedním Undo,
které lze samostatně upravovat, seskupit a exportovat do všech cílů.

### Schody s podestou a zatočením

U schodů lze zvolit přímé rameno, zatáčení vlevo/vpravo do L nebo návrat do U.
Přímé rameno má volitelnou **Podestu**, otočené varianty **Šířku ramene**.
**Tloušťka spodní desky** nula vytváří plné schody; kladná hodnota souvislou
šikmou desku s tloušťkou měřenou svisle. Podesta na desku navazuje. Rozšířené
varianty mají rovnoměrnou výšku stupňů; celkem lze vytvořit nejvýše 512 stupňů.

## Výběr
Kliknutím brush vyberete, `Ctrl`-kliknutím přidáte do výběru. Tažení z prázdného místa kreslí výběrový obdélník. Ve 3D pohledu `Shift`-kliknutí vybere jednu stěnu a `Ctrl+Shift`-kliknutí přidá další stěny. Tažením vybraného brushe ho přesunete; tažením nevybraného vyberete a přesunete jen ten.

Ve 3D pohledu `Shift`-tažením stěny vybraného brushe stěnu vytáhnete. `Alt`-kliknutím na jinou stěnu na ni přenesete materiál a zarovnání vybrané stěny; `Alt+Ctrl` přenese jen materiál a `Alt+Shift` materiál s UV otočeným podle cílové stěny.

Pokud správce oken používá `Alt` pro přesouvání oken, použijte `Ctrl+C` na zdrojové stěně, vyberte cílové stěny a stiskněte `Ctrl+V`. Kopírování a vložení funguje i pro úchyty nástroje Stěna. Přenos materiálu odstraní cílový barevný nátěr, aby byla textura viditelná.

## Tvarovací nástroje
- **Vybrat** (`Esc`): výběr a posun.
- **Vrchol** (`V`), **Hrana** (`E`), **Stěna** (`F`): tažení rohů, hran nebo stěn. `Shift`-tažení ze středu hrany nebo stěny tam přidá roh. `Delete` odebere vybrané rohy.
- **Otočit** (`R`), **Měřítko** (`T`), **Zkosit** (`G`): transformace výběru pomocí úchytů.
- **Řez** (`C`): dvěma nebo třemi kliknutími určete řeznou rovinu. `Enter` řízne, `Ctrl+Enter` volí, která strana zůstane, a `Backspace` nebo `Esc` odebere poslední bod.
- **Skládat** (`N`): rozmístěte body kolem objemu a `Enter` z nich vytvoří konvexní brush. `Backspace` odebere poslední bod.
- **Protažení** (`Y`): viz sekce o protažení níže.
- **Řídicí bod** (`P`): úprava řídicí mřížky plátu.
- **Zaoblení** (`O`): kliknutím vyberete hranu pro náhled. Upravte poloměr, počet ploch a rohy a potvrďte přes **Použít** nebo `Enter`. Kliknutí na tělo brushe ho vybere i s aktivním nástrojem; `Ctrl`-kliknutí na tělo přidá nebo odebere brush. Na hranách `Ctrl` přidává nebo odebírá hrany. Kliknutí do prázdna zruší výběr brushů i náhledu. Platí ve 2D i 3D. Více ploch dá hladší výsledek a více geometrie.
- **Plechovka** (`K`): viz Materiály a UV.

Nastavení všech kreslených tvarů otevřete tlačítkem **Nastavení tvaru** v liště. Výběr jiného tvaru tam okno otevře automaticky. Okno má svislé řádky a číselné volby s −/+, lze je přesunout a přitom pracovat v mapě. U rozpracovaného tvaru změny ihned upravují náhled; **Potvrdit** jej přijme, **Zrušit** odstraní vložení. **Zavřít** pouze skryje okno a ponechá tvar upravitelný, dokud nezměníte výběr.

## Kombinace brushů
- `Ctrl+J`: sloučit vybrané brushe do jejich konvexního obalu.
- `Ctrl+K`: odečíst vybrané brushe od brushů, kterých se dotýkají.
- `Ctrl+Shift+K`: vyhloubit výběr se stěnami silnými jeden krok mřížky.
- `Ctrl+L`: průnik výběru.

## Protažení, zaoblení, plát a zkosení
**Protažení** vytváří geometrii z vybraných stěn podél dráhy. Vyberte stěny pomocí `Shift`-kliknutí v nástroji Výběr, v sekci Protažení na kartě Stěna nastavte **Dráha**, **Otočení** (osa a úhel), **Měřítko**, **Kroky** a **Cesta** (zarovnání Celočíselné nebo Volné; Oblouk, Přímá nebo Esovitý ohyb) a stiskněte **Protažení**. Řádek pod ovládáním říká, kolik brushů vznikne. Protažení zachovává materiál,
barvu a PBR nastavení zdrojových stěn, navazuje UV přes společné boční hrany
a skrývá vnitřní uzávěry. Zhroucení segmentu nebo objemový samoprůnik odmítne
celý výsledek. Nové díly zůstávají ve stejných entitách/skupinách jako zdroj,
mají jeden krok Undo a celkem je omezuje limit 1024 buněk.

Sekce **Zaoblení** zaoblí vybrané hrany podle hodnot **Poloměr**, **Stěny** a **Rohy** (Zaříznuté nebo Zaoblené); jedna plocha znamená sražení hrany. Poloměr i počet segmentů mají tlačítka −/+.
**Vnitřní roh dvou ramen L** přidává výplň mezi dva překrývající se obdélníkové
brushe se stejnou hloubkou a rodičem, včetně různých šířek ramen. Vnější
3D náhled zaoblení zobrazuje výsledné plochy a zvýrazněné hrany včetně
vnitřní výplně. Změna poloměru nebo segmentů náhled aktualizuje před potvrzením
přes Použít nebo Enter. Změna nástroje vrátí původní zobrazení bez úpravy mapy.
Fillet odvíjí UV přes své segmenty; u stejného materiálu upraví i navazující
rovnou stěnu. Kulové styky více hran mohou mít samostatné UV švy.

**Plát** ukazuje **Sloupce** a **Řádky** vybraného plátu s tlačítky pro přidání nebo odebrání dvojice. Geometrie → Převést stěny na pláty (`Ctrl+P`) vytvoří pláty z vybraných stěn.

**Zapuštění a zkosení** pracuje s vybranými stěnami: **Zapuštění** stáhne stěnu od jejích hran dovnitř a **Hloubka** ji vysune ven (nebo dovnitř, je-li záporná). **Zkosit stěny** použije obojí; třetí tlačítko přidá panel nebo vyřízne výklenek podle znaménka hloubky. **Spojit roh** otevře dialog, který spojí dva brushe zaříznutým nebo zaobleným rohem s náhledem.

V dialogu **Spojit roh** tlačítka −/+ odmítají kroky mimo skutečné geometrické
meze. Neplatné ruční zadání ponechá poslední platný náhled. Režim
**Full-thickness wedges** nabízí **Zachovat tloušťku stěny**: posune krajní
kontakty tak, aby jednotlivé rovné segmenty měly správnou kolmou tloušťku.
U různě širokých ramen se tloušťka interpoluje. Kolečko v náhledu přibližuje,
tažení posouvá a dvojklik nebo **Obnovit pohled** vrátí celý roh do záběru.
Oba náhledy sdílejí stejné přiblížení i posun.

**Spojená oblast stěn** upravuje společnou hranici vybraných stěn současně,
bez dvojího stažení jejich společné hrany. Podporuje i celé sdílené hrany
stěn ve stejné rovině na různých brushích. Selhání jedné části odmítne celou
operaci; vypnutím volby lze upravovat stěny jednotlivě.

Ve stejném panelu jsou nové volby pro **Vytvořit dutinu**: tloušťka **Dovnitř**,
**Ven** nebo **Na obě strany**, otevření vybraných stěn a společný plášť dvou
brushů tvořících konvexní objem pod stejným rodičem. Vybraná stěna se otevře
bez ponechaného víka. Společný plášť odstraní i původní dělící stěnu.
Zkratka Hollow nadále používá jeden krok mřížky směrem dovnitř.

Po nakreslení zůstává tvar v dočasném **Náhledu tvaru**, zvýrazněný barvou
úchytů změny velikosti. Volby nástroje Brush nadále mění právě vložený tvar,
včetně duté koule, kužele, schodů a ostatních generátorů. **Potvrdit** nebo
`Enter` ukončí úpravu parametrů; **Zrušit** nebo `Escape` vezme vložení zpět.
Změna výběru, přepnutí nástroje či další geometrická úprava přijme aktuální
výsledek jako běžné brushe. Celé vložení včetně úprav parametrů má jedno Undo.
Neplatný parametr ponechá poslední platnou geometrii. Aktuální platná geometrie
se běžně ukládá i exportuje; parametry generátoru se po potvrzení ani po novém
otevření mapy neobnovují.

## 3D text
Geometrie → Vložit 3D text (také poslední položka seznamu tvarů nástroje Brush) vytvoří brushe z textu. Používá Heron Block, vestavěné polygonové písmo se sraženými rohy, proporcionálními šířkami a českou diakritikou; malá písmena mají tvar velkých. Napište **Text**, nastavte **Výška** (výška verzálek; diakritika sahá výš) a **Hloubka**, sledujte živý náhled a stiskněte **Vložit**.

Pokud operace nemůže vytvořit platnou geometrii, upravte rozměry, nebo vraťte změnu a tvar zjednodušte.

## Pole
Geometrie → Pole zkopíruje výběr a kopie ukazuje v mapě, zatímco měníte hodnoty. Počty zahrnují originál a pole vytvoří nejvýš 1024 kopií.

- **Řada** posune každou kopii oproti předchozí o **Krok X**, **Krok Y** a **Krok Z** a otočí ji o **Otočení na kopii (stupně)** kolem zvolené osy.
- **Mřížka** vyplní **Počet X**, **Počet Y** a **Počet Z** položek s rozestupy **Krok X**, **Krok Y** a **Krok Z**.
- **Kruh** rozmístí položky v rozsahu **Úhel (stupně)** kolem středu vzdáleného **Poloměr (jednotky)** od výběru. Se zapnutým **Natočit ke středu** se každá kopie otáčí s kruhem.
- **Rozptyl** umístí kopie na náhodná místa v daném poloměru napříč zvolenou osou. Stejný **Seed** dá stejné rozmístění a **Náhodné otočení** každou kopii otočí.

Když je vybraná jedna celá skupina, **Propojené kopie** udělá z každé kopie propojený duplikát, takže úprava jedné se projeví ve všech. **Použít** kopie ponechá a vybere je spolu s originálem; **Zrušit** je odstraní. Celé pole se vrací jedním krokem zpět.

## Sochání terénu
Terén tvoří mnoho malých brushů, jejichž vršky sochařský štětec posouvá nahoru a dolů. Vyberte plochý kvádr a zvolte Geometrie → Rozdělit na sloupce: kvádr se rozřeže na trojboké sloupce široké jeden krok mřížky a vloží je do skupiny Terrain. Vršek trojúhelníku je vždy rovný, takže sloupce mohou tvořit hřebeny i údolí.

Zvolte nástroj **Sochat** (`J`) a táhněte po terénu ve 3D pohledu. Oranžový kruh ukazuje štětec. Bez výběru štětec mění všechny brushe pod sebou, s výběrem jen ty vybrané. Lišta nahoře v okně určuje, co tah udělá:

- **Zvednout** a **Snížit** posunou povrch nahoru nebo dolů, nejvíc uprostřed kruhu a na okraji vůbec. Podržením `Shift` je prohodíte.
- **Vyhladit** srovná hrbolky.
- **Zarovnat** zarovná povrch na výšku, kde tah začal.

Dvě čísla vedle režimů jsou **Poloměr (jednotky)** a **Síla (%)**. Posouvají se jen horní rohy, a to svisle; spodek zůstává na místě a sloupec zůstane aspoň jednu jednotku silný. Tah se vrací jedním krokem zpět. Ve 2D pohledech nástroj vybírá stejně jako nástroj Výběr.

# Materiály a UV
## Načtení a použití materiálů
Otevřete sekci **Materiály** na kartě Stěna. **Načíst složku** přidá složku obrázků a **Načíst .wad** balík textur; oba se objeví v **Sady**, kde je křížek odebere. Psaním do **Filtr materiálů** zúžíte seznam. Kliknutí zvolí aktuální materiál a rovnou ho použije na vybrané stěny nebo brushe. Bez výběru pouze změní aktuální materiál. **Použít na výběr** jej znovu nanese. Použití textury odstraní barevný nátěr či přechod na těchto stěnách, jejich UV mapování zachová. **Vybrat** nabízí Vybrat brushe s ním a Vybrat stěny s ním. **Nahradit** zamění jeden materiál za jiný v celé mapě nebo jen ve výběru.

Nástroj **Plechovka** (`K`) dá aktuální materiál na stěnu pod ukazatelem. Držte levé tlačítko a táhněte přes stěny pro průběžné nanášení; celý tah vrátíte jedním Undo. Funguje ve 2D i 3D, zachovává UV a odstraní nátěr zakrývající texturu. Kliknutí obarví jednu stěnu, dvojklik celý brush a `Shift`-kliknutí převezme materiál jako kapátko.

Po `Ctrl+C` na zdrojové stěně vložíte její materiál a UV **prostředním kliknutím** na cílovou stěnu ve 2D i 3D, bez změny výběru. Tažení prostředním tlačítkem dál posouvá pohled. Obsah schránky s celými objekty se tímto gestem nevkládá.

## Editor UV
UV editor lze otevřít přes **Zobrazení → UV editor v samostatném okně** nebo
ikonou šipky ven ze čtverce v sekci UV. Popisek se zobrazí po najetí myší.
Okno má větší plátno a všechny původní
ovládací prvky; výběr stěn i dokument sdílí s mapou. **Vrátit do hlavního okna**
vrátí editor do karty Stěna a případně znovu zobrazí inspektor. Na desktopu jde
o samostatné okno, ve webu o plovoucí dialog. Plátno se při změně velikosti
znovu přizpůsobí stěně. Tlačítka **Zpět/Znovu** pracují se společnou historií;
v nativním okně lze použít i `Ctrl+Z`, `Ctrl+Shift+Z` a `Ctrl+Y`.

Sekce **UV** na kartě Stěna ukazuje materiál a obrys vybrané stěny. Ovládání myší na plátně:
- Kolečko přibližuje kolem ukazatele. Tažením pravým nebo prostředním tlačítkem posouváte. Dvojklikem pravým tlačítkem stěnu znovu zarámujete; výběr jiné stěny ji zarámuje automaticky.
- Tažení levým tlačítkem posouvá materiál, obrys stěny zůstává na místě. Kruh otáčí mapováním, rohové úchyty mění měřítko a středový úchyt určuje počátek těchto změn.
- `Alt`-tažení levým tlačítkem za okraj dlaždice zkosí, stejně jako `Shift`-tažení prostředním tlačítkem.
- Každé tažení je jeden krok zpět a `Esc` probíhající tažení zruší.

Pod plátnem jsou **Posun**, **Měřítko**, **Úhel** a, pokud je hra používá, **Příznaky obsahu**, **Příznaky povrchu** a **Hodnota povrchu**. Tlačítka jsou **Vynulovat**, **Zarovnat podle světa**, **Přizpůsobit**, **Překlopit vodorovně**, **Překlopit svisle** a dvě otočení o 90°. **Zarovnat** natočí materiál podél jedné z hran stěny (dalším stiskem další hrana, se `Shift` předchozí). Čtyři tlačítka zarovnání přisunou materiál k hraně. **Vyplnit U** a **Vyplnit V** změní měřítko tak, aby stěnu pokryl celý počet opakování; dalším stiskem další počet, s `Ctrl` se zachová dělení trim sheetu.

## Klávesy pro UV
V hlavním okně s vybranými stěnami šipky posouvají materiál, `[` a `]` ho otáčejí o 15°, `,` a `.` jeho měřítko zmenší na polovinu nebo zdvojnásobí, `Ctrl+F` a `Ctrl+Alt+F` ho překlopí, `Shift+R` ho vynuluje, `Alt+Shift+R` ho vynuluje podle os světa a `Ctrl+Shift+F` ho přizpůsobí každé stěně.

**Zámek** na liště nástrojů zachová mapování při posunu brushů a **UV** (`U`) při jejich otáčení.

## Materiály nástrojů
- skip: vyřadí označenou stěnu z exportu do enginů.
- clip: vytvoří kolizi bez viditelné geometrie.
- origin: označí pivot brushové entity.

Názvy lze změnit v kartě **Materiály nástrojů** v okně Export. Skip používejte pro stěny, o kterých víte, že jsou skryté.

## Vynechání stěn z exportu
Vyberte stěnu a v sekci UV zapněte **Vynechat stěnu při exportu**. Oranžové šrafování ji označí v náhledu UV. Materiál i barva zůstanou uložené a brush zůstane uzavřený a upravitelný. Exportované vykreslovací meshe stěnu vynechají, kolize zůstane. Ponecháte-li z krychle jeden čtyřúhelník, vzniknou dva vykreslovací trojúhelníky. Volba nezapíná oboustranné vykreslování ani slučování meshů.

**Vynechat zakryté stěny** označí stěny zcela zakryté statickými brushi, včetně shodných dotýkajících se stěn a uzavřených brushů. Pracuje s vybranými brushi, nebo se vší viditelnou a nezamčenou geometrií světa, když není nic vybráno. Brushové entity, průhledné materiály a brushe, které už vynechané stěny mají, se přeskočí. Částečně zakryté stěny se nedělí. Jedno vrácení obnoví vše. Zobrazení → Zobrazit vynechané stěny takové stěny v pohledech editoru skryje nebo ukáže; výběr stále pracuje s celým brushem.

# Barvy a přechody
Sekce **Barva** na kartě Stěna barví stěny jednolitými barvami nebo přechody přes jejich materiály. Hodí se pro hrubý návrh úrovně i pro stylizovanou grafiku. Obarvené stěny si v pohledech ponechají skutečné barvy; výběr je vyznačen obrysem.

## Paleta a míchaná barva
Kliknutím na jeden z dvanácti vzorků palety obarvíte vybrané stěny nebo celé vybrané brushe. **Krytí** nastavuje v procentech, jak krycí bude další barva. **Míchat** má barevné tlačítko, které otevře výběr barvy; při tažení ve výběru se výběr mění živě, nebo tlačítkem **Plechovka** míchanou barvu použijete jako jeden krok. **Zpět na materiál** barvu odstraní a stěna znovu ukáže materiál. Řádek nahoře v sekci říká, co je vybráno a čím je to obarveno.

## Výběr barvy
Každé barevné tlačítko v editoru otevírá stejný výběr barvy. Tažením ve čtverci volíte sytost a jas, v pruhu vedle něj odstín. Pod nimi jsou hodnoty **R**, **G** a **B** a posuvník **Krytí**, pokud barva krytí má. Pole s hexadecimálním kódem přijímá `#RRGGBB` nebo `#RRGGBBAA`; dokud text není platná barva, je červené. **Kopírovat** vloží kód do schránky. Kliknutím mimo výběr ho zavřete.

## Přechody
**Přechod** volí Lineární gradient nebo Radiální gradient a tlačítko vedle prohodí barvy konců. Každý bod má barevné tlačítko a **Poloha (%)**, body mezi konci mají i tlačítko pro odebrání. **Přidat barevný bod** přidá bod do největší mezery; přechod může mít až osm bodů. **Úhel přechodu** určuje směr lineárního přechodu, s tlačítky **+45°** a **−45°**.

Pod náhledem je lišta se značkou pro každý bod. Kliknutím na lištu přidáte bod, kliknutím na značku otevřete její výběr barvy, tažením značku posunete a kliknutím pravým tlačítkem ji odeberete. Tři vzorky pod lištou jsou předvolby: Teplý přechod, Studený přechod a Přechod ve stupních šedi. Změny přechodu se na výběr projeví živě.

Na směru přechodu záleží: veďte ho podél sklonu střechy nebo zakřiveného objektu, ne napříč nesouvisejícími stěnami. Po obarvení model zkontrolujte z obou stran.

## Materiál povrchu
**Materiál povrchu** nastavuje **Drsnost** (0 je lesklý, 100 matný), **Kovovost** (0 pro dřevo, kámen nebo plast, 100 pro kov) a **Emise** (0 až 8; povrch září vlastní barvou, včetně přechodů, ale okolní objekty neosvětluje). Změny se na vybrané stěny nebo brushe použijí živě. **Výchozí povrch** přepsání odstraní. Všechny změny lze vrátit.

Efekty povrchu jsou vidět v osvětleném 3D pohledu a ve vykreslení kamerou. Plné a drátové zobrazení odlesky záměrně vynechává. Bloom může kolem zářících povrchů přidat záři. Godot a glTF/GLB dostanou hodnoty materiálu, importér pro Unity převede drsnost na hladkost. Omezení jednotlivých cílů popisuje `docs/surface-materials.md`.

# Assety, vrstvy a skupiny
## Knihovna assetů
Otevřete kartu **Assety**. Složku modelů `.map` přidáte tlačítkem **Přidat složku**; vestavěná knihovna je k dispozici vždy. Psaním do **Filtrovat assety** hledáte. Kliknutím model vyberete, dvojklikem, klávesou `Enter` nebo tlačítkem **Vložit** ho přidáte tam, kde byl naposledy ukazatel. Model můžete také přetáhnout do pohledu. Brushe assetu se vkládají jako jedna skupina, takže se model pohybuje jako celek.

**Uložit výběr jako asset** zapíše vybrané brushe do knihovny jako `.map`. Assety v knihovně jsou běžné upravitelné brushe, ne importované meshe.

## Vrstvy
Seznam **Vrstvy** na kartě Mapa organizuje velké oblasti. Napište název a kliknutím na **Přidat** vytvořte vrstvu, nebo klávesou `Enter` přejmenujte aktuální. Každou vrstvu lze skrýt, zamknout nebo vynechat z exportu. **Přesunout sem** přesune výběr do aktuální vrstvy. Místní nabídka v pohledech nabízí také Přesunout do vrstvy, Nastavit jako aktivní vrstvu, Skrýt vrstvy a Izolovat vrstvy.

## Skupiny a provázané skupiny
`Ctrl+G` výběr seskupí a `Ctrl+Shift+G` skupinu zruší. Sekce **Skupiny** na kartě Mapa skupiny vypisuje a nabízí **Otevřít**, **Přejmenovat**, **Sloučit**, **Zrušit skupinu**, **Vyjmout** a **Vyčlenit**. Provázaná skupina je sada kopií, které sdílejí úpravy: `Ctrl+Shift+D` vytvoří provázanou kopii, Geometrie → Sladit provázané skupiny kopie aktualizuje a **Zrušit provázání** udělá z kopie samostatnou skupinu. Na kartě Entita lze vlastnost ponechat jako vlastní jedné kopii, když se provázané skupiny slaďují.

## Společné měřítko
Výchozí měřítko exportu je 32 jednotek mapy na metr. Vestavěná značka startu hráče je vysoká 64 jednotek, tedy 2 metry. Porovnávejte s ní dveře, vozidla, nábytek i rostliny. Měřítko exportu měňte jen záměrně a související assety držte konzistentní.

# Entity a hratelnost
Běžné definice entit jsou vestavěné a vždy k dispozici: světla, starty hráče, dveře, tlačítka, plošiny, spouštěče, statické rekvizity, zvuky a body trasy. Načtení FGD projektu přes **Definice entit** na kartě Entita nahradí třídy se stejným názvem a ostatní vestavěné ponechá.

## Umisťování entit
V prohlížeči **Entity** na kartě Entita napište do **Filtr tříd** a zvolte třídu. Bodovou třídu, například světlo, umístíte tlačítkem **Umístit** nebo dvojklikem na třídu; `Insert` jednu umístí do středu výběru. U brushové třídy, například dveří nebo spouštěče, vyberte brushe a klikněte na **Vytvořit**. Třídy lze také přetáhnout do pohledu.

## Vlastnosti
Seznam vlastností upravuje vybranou entitu. Entity stejné třídy se upravují společně. **Přidat vlastnost** přidá klíč, který definice nemusí znát; **Výchozí** umí vrátit existující vlastnosti na výchozí hodnoty nebo doplnit chybějící. Vlastnosti, které třída definuje, ale entita je nemá, jsou zobrazeny tlumeně; zadáním hodnoty se přidají. Pro propojení entit použijte targetname a target.

## Co umí export
Direktivy pro Godot vytvoří nativní uzly, kolizní tělesa a oblasti spouštěčů. Unity a Unreal podporují nativní běžná světla, Unreal navíc vytváří aktéry PlayerStart. Ostatní data se zachovají, nakolik to importér cíle dovolí.

Samotná entita dveří nepřidá herní chování. Pohyb, reakce spouštěčů, ovládání hráče a skripty projektu je stále potřeba udělat v enginu. Defold exportuje vlastnosti entit a kolizní objekty; Flax umisťuje modely a obecné entity. Než na výsledku postavíte hratelnost, zkontrolujte ho v enginu.

# Animace
Heron Brush umí zobrazit náhled jednoduchého pohybu, upravovat animace brushových entit klíčovými snímky a exportovat je do několika enginů.

## Náhled animací
Zapněte **Náhled animací** v nabídce Zobrazení, v nabídce Animace, v nabídce Nastavení mřížky nebo v sekci Zobrazení na kartě Mapa. Přehrává rotaci func_rotate a pohyb tam a zpět func_shuttle, je-li zapnuté start_on, podle vlastností osy, rychlosti, směru, dráhy a čekání, a přehrává klipy klíčových snímků nastavené na Přehrát při startu. Assety větráku a větrného mlýna už otáčející se části obsahují.

Vypnutí náhledu obnoví původní pózu. Kliknutí do pohledu zastaví přehrávání ještě před výběrem nebo úpravou; ovládání kamery funguje dál. Náhled nemění mapu, historii úprav ani exporty a nespouští fyziku ani herní logiku.

## Editor klíčových snímků
Otevřete ho přes Animace → Animace klíčovými snímky nebo z nabídky Nastavení mřížky. Ukotví se pod pohledy, nebo do 3D okna, pokud je samostatné. Ikona okna v jeho záhlaví ho přesune do vlastního okna (**Editor animací v samostatném okně**); snímek i historie úprav zůstanou. **Upravovat animaci** přepne pohledy na animovanou pózu a **Upravovat mapu** se vrátí ke klidové geometrii. V režimu animace každý pohled dole ukazuje **Upravovat animaci** a číslo snímku.

Animovat můžete vybranou brushovou entitu, brush nebo skupinu brushů. Plochá skupina brushů sdílí jednu animaci a pivot; skupiny s více entitami potřebují samostatné animace.

## Klipy
Výběr **Animace** přepíná mezi pojmenovanými klipy. Napište název do **Název animace** a tlačítkem **Uložit jako** aktuální klip pod tímto názvem zkopírujete; **Smazat** klip odstraní. **Posun** vytvoří klip `translate` bez rotace. **Animace dveří** vytvoří plynulé klipy `open` a `close` s otočením o 90 stupňů kolem aktuálního pivotu; pivot nejprve přesuňte do závěsu. **Přehrát při startu** spustí klip při startu úrovně; předvolba dveří tuto volbu vypíná.

## Ovládání přehrávání
Tlačítka přehrávání jsou **Začátek**, **Předchozí klíč**, **Přehrávání**, **Další klíč** a **Konec**. **Snímek** ukazuje a nastavuje aktuální snímek a **Délka (s)** délku klipu. Režim je **Jednou**, **Smyčka** nebo **Tam a zpět**.

## Časová osa
Časová osa má pravítko s přehrávací hlavou a stopu pro **Poloha** (X, Y a Z s klíči dohromady) a pro každý úhel rotace, RX, RY a RZ. Kliknutím nebo tažením v pravítku posunete přehrávací hlavu a ve všech pohledech uvidíte pózu v daném čase. Tažením prostředním tlačítkem časovou osu posouváte. Klíče v čase přesouváte tažením; `Shift` přidává do výběru; dvojklik do stopy vloží klíč; `Ctrl`+kolečko přibližuje. **Křivky** místo stop ukáže křivky hodnot.

Nad ní jsou tlačítka **Přidat klíč**, **Odebrat**, **Kopírovat**, **Vložit** a **Duplikovat**. **Zarovnat na snímky** drží klíče na celých snímcích a **Přizpůsobit** ukáže celý klip. **Upravit časování** otevře **Časové měřítko**, **Posun (snímky)**, **Měřítko od přehrávací hlavy**, **Prodloužit animaci** a **Upravit časování výběru**.

Inspektor klíčů vedle časové osy upravuje vybraný klíč: **Snímek**, **Hodnota** (nebo tři souřadnice klíče polohy), interpolaci **Lineární**, **Skoková** nebo **Bézierova**, **Vstupní sklon** a **Výstupní sklon** a **Vyhladit tečny**.

## Klávesy v editoru
Když má klávesnici časová osa: `Space` spustí nebo pozastaví přehrávání, šipky vlevo a vpravo posunou o snímek, `Ctrl+A` vybere všechny klíče, `Ctrl+C`, `Ctrl+X` a `Ctrl+V` klíče kopírují, vyjímají a vkládají, `Ctrl+D` je duplikuje, `Delete` je odebere, `Ctrl+Z` vrací a `Esc` zruší tažení. Mimo časovou osu `Esc` opustí režim animace. Vrácení změny zachová přiblížení, výběr i polohu přehrávací hlavy.

## Pózování v pohledech
Řádek pod časovou osou obsahuje **Onion skinning** s počtem a **Rozestup**, které ukazují sousední pózy jako průsvitné obrysy; souřadnice **Pivot**; **Automatické klíče**; a nástroje pózy **Posun**, **Otočit** a **Pivot**.

- **Přidat klíč** zapne Automatické klíče a pozastaví přehrávání. Se zapnutými Automatickými klíči a zvoleným **Posun** táhněte ve 3D pohledu barevnou osu úchytu vybrané entity a v aktuálním snímku tím zaznamenáte polohu.
- Ve 2D pohledu táhněte vybraný brush nebo člena jeho skupiny a zaznamenáte posun podél dvou viditelných os s přichytáváním k mřížce. Automatické klíče k tomu zapnuté být nemusí.
- S **Otočit** táhněte brush ve 2D pohledu kolem jeho oranžové značky pivotu. Pohled Shora zaznamenává RZ, Zepředu RY a Zboku RX, s přichytáváním úhlu podle mřížky.
- S **Pivot** táhněte žlutý střed otáčení v libovolném 2D pohledu. Přichytává se k mřížce a zachovává hloubku. Klíče ani klidová geometrie se nemění.

Azurová dráha ve 2D i 3D ukazuje pohyb pivotu. Kliknutím na bod dráhy vyberete jeho čas a klíče, tažením změníte polohu; se zapnutými Automatickými klíči lze body dráhy táhnout i ve 3D pohledu. Popisky ukazují čísla snímků a čárkované úseky značí skoky. `Esc` kterékoli z těchto tažení zruší a jedno vrácení vrátí celé tažení.

Nové animace začínají v klidu s klíči v čase nula. Pohyb vytvoříte změnou pózy v jiném snímku. Pro ostrou dráhu ve tvaru L zaznamenejte začátek, roh a konec a použijte Lineární s Tam a zpět.

## Propojení tlačítek s animacemi
Vyberte func_button, trigger_multiple nebo trigger_once. Karta Entita ukáže nad vlastnostmi **Spouštění animace**. Vyberte cíl ze seznamu, nebo klikněte na **Vybrat cíl v pohledu** a pak na animovaný objekt v libovolném pohledu; `Esc` výběr zruší. Chybějící cíle nebo klipy se vypíší pod ovládáním. Je-li zapnuté zobrazení propojení, entity ukazují šipky ke svým cílům.

**Akce** volí, co spuštění udělá: **Přehrát** spustí pojmenovaný klip nebo vybranou animaci; **Přepnout** střídá `open` a `close`, pokud existují, jinak znovu spustí vybranou animaci; **Otevřít** a **Zavřít** spustí tyto klipy; **Pozastavit** podrží pózu, **Pokračovat** pokračuje a **Zastavit** obnoví původní pózu. **Pouze jednou** přepíše výchozí chování třídy. **Interval opakování (s)** nastaví nejkratší dobu mezi spuštěními; první spuštění nezdržuje.

**Vyzkoušet spuštění** provede akci v náhledu bez úprav mapy a bez kroků zpět. **Obnovit náhled** obnoví původní pózy a vynuluje stav jednorázového spuštění a prodlevy. Kolize hráče ani herní skripty se nesimulují.

## Animace v exportech
Godot exportuje nativní zdroje AnimationPlayer a AnimationLibrary, včetně křivek a pohyblivé kolize. CharacterBody3D, který vstoupí do oblasti tlačítka, spustí jeho cíl. Úpravy těchto klipů v Godotu se do mapy nevrátí a další export je nahradí.

Export pro raylib přidá `<map>_animation.h` s přehrávačem v C a `<map>_preview.c` s náhledem s volnou kamerou; sestavení popisuje `<map>_raylib.md`. V náhledu `SPACE` pozastaví, `R` znovu spustí výchozí klipy a `N` přepíná pojmenované klipy. Defold vkládá přehrávání do skriptu entity se zdokumentovanými zprávami `heron_*_animation`. Build Level v Unity přidá `HeronBrushAnimation` a kinematické těleso. Export pro Bevy obsahuje `heron_animation.rs` a návod. Godot, Unity, Defold, Bevy a raylib zachovávají propojení spouštěčů, jednorázovost i interval opakování. Vaše hra ale stále musí zaregistrovat hráče a připojit vlastní tlačítka k API každého běhového prostředí.

## Příklad Motion lab
Otevřete `motion_lab` v sekci Příklady na uvítací obrazovce. Obsahuje plošinu s dráhou ve tvaru L, výtah, otáčivou plošinu a dveře na pantech s tlačítky. Otevřete vedlejší `godot/project.godot` v Godotu a stiskněte `F5`. WASD a myš ovládají hráče, mezerník skáče, 1 až 4 přenesou na stanoviště a R restartuje aktuální stanoviště. Šlápnutím na barevnou plošku spustíte její cíl. Přiložené README popisuje opakovaný export.

# Export do enginu
Soubor → Export otevře okno Export. Vlevo jsou enginy s logy: Godot, Defold, three.js, Bevy, raylib, Unity, Unreal Engine, Flax Engine, Cocos Creator, Wavefront .obj, glTF .gltf, GLB .glb a nakonec `.map`. Stránka vpravo říká, co cíl dostane. **Zapsat** ho zapíše, **Zkontrolovat** úroveň sestaví bez zápisu a vypíše problémy a **Zavřít** okno zavře.

Export je oddělený od ukládání: `.map` zůstává vaším upravitelným zdrojem. Každý ovládací prvek ukládá nastavení svého enginu hned při změně, takže si každý engine pamatuje vlastní volby a okno se otevře tak, jak jste ho opustili.

## Společné volby
- **Měřítko**: **Jednotek na metr**, výchozí 32.
- **Co zapsat**: **Kolizní geometrie**.
- **Optimalizace**: **Seskupit statické meshe do buněk o 1024 jednotkách**, **Zahodit stěny pohřbené uvnitř geometrie**, **Spojit stěny ve stejné rovině** a **Spojit kolizní tvary**.
- **Cesty projektu**: kam se v projektu zapíše úroveň a její zdroje.
- **Materiály nástrojů**: názvy materiálů clip, skip a origin.
- **Další**: přepínače podle enginu, například **Pojmenovat uzly podle**, **Obnovit vrstvy a skupiny jako nadřazené uzly**, **Obrátit pořadí vrcholů trojúhelníků**, převrácení texturových souřadnic a druhá sada UV pro zapečené světlo s **Texel světelné mapy**.
- **Průběžně podle mapy**: **Znovu zapsat při každém uložení mapy**. Je dostupné, až engine jednou exportujete, protože teprve tento export určí, kam zapisovat.

Statické buňky přiřadí brushe světa podle jejich středů a v každé buňce o 1024 jednotkách je spojí podle materiálu, takže enginy mohou vyřazovat menší části mapy. Kolize zůstane a herní entity zůstanou oddělené. Může to znamenat více vykreslovacích volání než jeden mesh světa, proto je volba ve výchozím stavu vypnutá. Samostatné exporty OBJ, glTF a GLB buňky nepoužívají.

## Poznámky k cílům
- Godot: cesty `res://` k texturám a materiálům, fyzikální vrstva spouštěčů a lightmap UV2. Zvolte složku exportu uvnitř projektu Godot.
- Defold: adresář zdrojů assetů, materiál modelu a sampler textury, který musí ve zvoleném materiálu existovat.
- Unity: meshe OBJ a importér pro editor; exportujte do složky Assets projektu.
- Unreal Engine: vygenerovaný importní skript a cesta k obsahu, obvykle /Game/<level>.
- Flax Engine: cesta k modelům relativní ke složce Content projektu.
- three.js, Bevy, raylib a Cocos Creator: scény nebo kód pro tato prostředí s návodem vedle exportu.
- glTF / GLB: přenositelná geometrie pro kompatibilní nástroje; GLB je binární podoba. OBJ: geometrie a materiály pro modelovací nástroje.
- `.map`: **Zapsat jako** volí Valve 220, Standardní, Quake 2 nebo Quake 2 (Valve). Okno řekne, co by zvolená varianta formátu ztratila.

Po zápisu si přečtěte varování, například o chybějících texturách. **Výsledky kontroly** je vypíše přímo v okně.

# Živé propojení: Godot, Unity, Defold a Flax
Živé propojení posílá neuložené úpravy z Heron Brush do editoru enginu během práce. Je jednosměrné: úpravy provedené v enginu se nevracejí. V jednom okně mapy může běžet jedno živé propojení. Před přepnutím enginu ho zastavte a po změně voleb exportu ho spusťte znovu. Ovládání najdete v okně Export na stránce enginu, v kartě **Živé propojení s**.

Indikátor v titulkovém pruhu ukazuje stav propojení; kliknutím ho zastavíte. Zavření nebo nahrazení mapy publikování ukončí. Po chybě importu problém opravte a propojení spusťte znovu.

## Godot
1. Jednou exportujte do projektu Godot.
2. Klikněte na **Nainstalovat doplněk Godotu**.
3. V Godotu zapněte Heron Brush Live v Project Settings → Plugins.
4. Klikněte na **Spustit živé propojení** a v doku Godotu připojte mapu k otevřené 3D scéně.

Doplněk nahrazuje jen svou vygenerovanou větev a zachová umístění připojení i ostatní uzly. Scénu v Godotu ukládejte sami a vlastní skripty a uzly držte mimo vygenerovanou větev.

## Unity
1. Jednou exportujte do složky Assets projektu Unity.
2. Klikněte na **Nainstalovat doplněk Unity** a nechte Unity doplněk zkompilovat.
3. V Unity otevřete Window → Heron Brush → Live Link.
4. Spusťte propojení a připojte mapu k aktivní scéně Unity.

Okno živého propojení v Unity nechte otevřené. Aktualizace se pozastaví v režimech Play Mode a Prefab Mode. Nahrazuje se jen potomek Generated objektu mapy. Vygenerované soubory neupravujte.

## Defold
1. Jednou exportujte do projektu Defold a projekt nechte v Defoldu otevřený.
2. Klikněte na **Spustit živé propojení**. Žádný doplněk není potřeba.
3. V Defoldu otevřete heron_brush_live/preview.collection, nebo ji použijte ve vlastní scéně.

Propojení zapisuje do samostatné vygenerované složky a finální export ani game.project nemění. **Průběžně obnovovat běžící hru** aktualizuje i hru spuštěnou z Defoldu; přidání objektů nebo změna kolizí může vyžadovat restart úrovně. Neúspěšná připojení se automaticky opakují.

## Flax
Exportujte do složky Content projektu a klikněte na **Nainstalovat doplněk pro Flax**. Nechte Flax doplněk zkompilovat, spusťte propojení a v otevřené scéně Flaxu zvolte Tools → Heron Brush: attach live map. Vygenerovaný kořen si zachová umístění a ostatní objekty zůstanou nedotčené. Scénu ukládejte sami. Aktualizace se v režimu Play Mode pozastaví. Tools → Heron Brush: stop live map příjem pozastaví.

# Společné úpravy v živé relaci
Spolupráce → Živá relace otevře okno pro hostování relace nebo připojení k jinému uživateli Heron Brush. Takto lidé sdílejí úpravy mapy; je to něco jiného než živé propojení s enginem.

## Hostování a připojení
Vyplňte **Vaše jméno**. Chcete-li hostovat, zvolte **Hostovat relaci**, klikněte na **Spustit relaci** a údaje z **Kopírovat údaje pro připojení** pošlete hostům. Chcete-li se připojit, zvolte **Připojit se k relaci**, vložte údaje a klikněte na **Připojit se**. Připojení nahradí otevřenou mapu mapou hostitele, proto nejprve uložte svou práci.

Standardní sestavení funguje v místní síti a **Port místní sítě** nastavuje port. Relace přes internet (**Připojit přes internet**) vyžadují sestavení s volitelnou síťovou funkcí. Účastníci ve 3D pohledu vidí značky kamer a jména ostatních.

Zavřením okna relace běží dál; její stav ukazuje indikátor v titulkovém pruhu. **Zastavit relaci** nebo **Opustit relaci** ji ukončí. Když dva lidé upravují stejný objekt, o konečné verzi rozhodne pořadí u hostitele, a i vrácení změny je sdílená úprava. Kde to jde, pracujte na oddělených oblastech a ukládejte si kontrolní body.

# 3D zobrazení a druhý monitor
## Režimy zobrazení
Sekce **Zobrazení** na kartě Mapa přepíná, co pohledy kreslí: Brushe, Bodové entity, Modely entit, Názvy tříd entit, Obrysy skupin, Patche, Materiály, Zobrazit vynechané stěny, Náhled animací, Drátový model, Hrany a Obrysy brushových entit. Drátový model kreslí hrany trojúhelníků včetně úhlopříček a hran za povrchy, bez materiálů. Je to dočasné ladicí zobrazení a nemá vliv na exporty ani vykreslení.

Osvětlení, stíny, ambient occlusion, odrazy, bloom a vyhlazování 3D pohledu najdete v Nastavení → Vykreslování. Jsou to nastavení náhledu; výsledné obrázky používají vlastní nastavení renderovací kamery.

## Samostatné 3D okno
Zobrazení → 3D pohled v samostatném okně přesune 3D pohled do vlastního okna, které můžete dát na jiný monitor. Sdílí kameru, dokument i nástroje a ploché pohledy zaberou místo v hlavním okně. Jeho nabídka obsahuje Materiály, Plné plochy a Drátový model; podnabídku Zobrazení s položkami Zobrazit vynechané stěny, Náhled animací, Hrany, Obrysy skupin, Bodové entity, Modely entit a Obrysy brushových entit; Zaostřit na výběr; Zepředu, Zboku a Shora, které natočí kameru kolem výběru (nebo kolem bodu před kamerou); Umístit kameru podle 3D pohledu; Celá obrazovka (`F11`); Vykreslit obrázek; a Vrátit do hlavního okna. Zavřením okna se pohled vrátí zpět.

## Procedurální obloha
Procedurální obloha je ve 3D pohledu ve výchozím stavu zapnutá a lze ji vypnout v Nastavení → Vykreslování → Světlo. Renderovací kamery mají vlastní přepínač **Procedurální obloha** uložený v mapě. Barvy oblohy, horizontu a země pocházejí z nastavení prostředí a slunce sleduje osvětlení scény. Obloha nemá geometrii a neexportuje se.

# Renderovací kamery a obrázky PNG
Soubor → Vykreslit obrázek otevře okno vykreslování: vlevo kamera a její nastavení, vpravo živý náhled, který se překreslí při každé změně. Kamery se ukládají do `.map` ve vrstvě Render cameras, která slouží jen editoru a do exportů pro enginy se nedostane.

## Umístění kamery
Nastavte záběr ve 3D pohledu a stiskněte `Ctrl+Alt+0` (nebo `Ctrl+Alt+Num0`), případně klikněte na **Umístit kameru podle 3D pohledu**. Není-li vybraná žádná kamera, aktualizuje se první uložená; neexistuje-li žádná, vytvoří se. Akci lze vrátit a zachová nastavení výstupu i osvětlení. **Vybrat kameru v mapě**, **Načíst kameru z mapy** a **Přesunout 3D pohled ke kameře** fungují opačným směrem. **Rámeček kamery ve 3D pohledu** ukáže hranici výstupu a **Ovládat kameru ze 3D pohledu** nechá běžnou navigaci hýbat uloženou kamerou.

## Nastavení
- **Poloha kamery**: X, Y, Z, natočení a sklon.
- **Výstup a objektiv**: **Předvolby** (Full HD, Čtverec, Na výšku), šířka, výška, vzorky, **Svislé zorné pole** a **Vzdálená ořezová rovina**.
- **Osvětlení**: Denní světlo, Západ slunce nebo Zataženo, případně expozice, slunce, barvy oblohy, horizontu a země, **Procedurální obloha**, **Stíny**, **Ambient occlusion** a **Bloom**.
- **Hloubka ostrosti**: **Zapnout hloubku ostrosti**, vzdálenost zaostření, šířka ostrého pásma a poloměr rozostření. **Zaostřit na výběr** zaostří na střed vybraného objektu před kamerou.

**Uložit kameru a nastavení** zapíše formulář do mapy; aby zůstal na disku, uložte mapu.

## Vykreslování
**Vykreslit PNG** se zeptá, kam uložit, a spustí postupné vykreslování, které se v náhledu doplňuje vzorek po vzorku. Více vzorků vypadá lépe, ale trvá déle. **Zrušit vykreslování** skončí bez zápisu souboru. Vykreslování používá rastrový postup editoru, ne sledování paprsků. PNG má neprůhledné pozadí a animace se nevykreslují. Velmi velké obrázky mohou editor na chvíli pozastavit.

# Nastavení
Soubor → Nastavení (`Ctrl+,`) otevře nastavení. Změny platí okamžitě a ukládají se. Postranní panel obsahuje Přehled, Hry, Zobrazení, Barvy, Myš, Klávesnice a Vykreslování. Pole pro hledání nahoře najde nastavení podle názvu nebo popisu na kterékoli stránce. Každé změněné nastavení má vlastní tlačítko pro návrat k výchozí hodnotě a **Obnovit výchozí** vrátí celou aktuální stránku.

## Stránky
- **Přehled**: běžná nastavení pohromadě: Vzhled (Krytí panelů (%), Barevné schéma, Rozvržení, Jazyk), Úpravy, Zobrazení a Příručka.
- **Hry**: hra, kde jsou její soubory a vaše jméno pro spolupráci. Hra určuje, jaký formát mapy zapisovat, které definice entit načíst a kde jsou její materiály. Konfigurace her se čtou ze složky `games` ve složce konfigurace (`~/.config/heron-brush/games` na Linuxu a macOS, `%LOCALAPPDATA%\heron-brush\games` na Windows) a ze složky `games` vedle aplikace, kam balíčky ukládají přibalenou konfiguraci.
- **Zobrazení**: Rozvržení, Barevné schéma, Jazyk, osy světa, krytí mřížky, velikost buňky kontroly netěsností a zámky mapování.
- **Barvy**: barvy pohledů, geometrie a úchytů.
- **Myš**: rychlosti a převrácení rozhlížení, posunu a pohybu, rychlost letu, zorný úhel, Provázat 2D pohledy a velikosti úchytů.
- **Klávesnice**: sada zkratek, Heron Brush nebo Blender.
- **Vykreslování**: realistické stínování, obrys výběru, světlo, stíny, ambient occlusion, odrazy, bloom a povrchy.

## Jazyk
**Jazyk** nabízí Podle systému a všechny přeložené jazyky. Rozhraní se změní okamžitě. Příručka se otevře v jazyce rozhraní, pokud pro něj existuje překlad, jinak anglicky.

# Pluginy a MCP
## Pluginy
Pluginy rozšiřují editor o skripty v jazyce Rhai. Soubor → Pluginy otevře okno Pluginy. **Nainstalovat plugin** nainstaluje soubor `.rhai` a **Nainstalovat složku pluginu** složku, která obsahuje `main.rhai`; volitelná metadata `plugin.toml` popisují plugin a jeho nastavení. Pluginy jsou rozdělené na **Aktivní pluginy** a **Neaktivní pluginy** a **Zapnout plugin** plugin zapne nebo vypne; volba přežije restart. **Znovu načíst pluginy** znovu přečte skripty z disku a vynuluje hodnoty v jejich panelech.

Aktivní pluginy se zobrazují také v sekci **Pluginy** na kartě Mapa s nastavením, tlačítky akcí (**Spustit**) a výstupem. Akce pracuje s výběrem, nebo se vším, když není nic vybráno, a je jedním krokem zpět. Přibalený plugin Selection inspector ukazuje počty a akce pro posun nahoru a dolů.

## MCP
Spolupráce → Zpřístupnit agentovi (MCP) umožní AI agentovi nebo jinému nástroji ovládat otevřený editor přes Model Context Protocol. Okno ukazuje místní adresu s tlačítkem **Kopírovat** a **Zastavit server** ho ukončí; dokud běží, titulkový pruh ukazuje **MCP aktivní**. Volba příkazové řádky `--mcp` spustí stejné rozhraní bez okna. Klienti mohou zjistit dostupné nástroje pro geometrii, assety, vlastnosti, animace, export a relace. MCP a pluginy editoru jsou dvě různá rozhraní.

# Příkazová řádka
Program se jmenuje `heron-brush`. Bez voleb otevře uvítací okno.

## Otevírání souborů
`heron-brush map.map [entities.fgd] [materials/]` otevře mapu, volitelně s definicemi entit a složkou materiálů. Prázdný argument jeden vynechá, například `heron-brush map.map "" materials/`. Pokud editor už běží, mapa se otevře v něm; `--new-instance` místo toho otevře samostatné okno.

## Příkazy
- `--help` nebo `-h`: vypíše použití. `--version` nebo `-V`: vypíše verzi.
- `--mcp`: obsluhuje MCP přes standardní vstup a výstup, bez okna.
- `--merge-map <base> <ours> <theirs> [<output>]`: třícestné slučování souborů `.map` pro git. Výstup je ve výchozím stavu ours. Konflikty se vypíší a vrátí nenulový návratový kód.
- `--obj-to-map <in.obj> <out.map> [size] [thickness] [snap] [material]`: převede mesh na brushe. Size je délka nejdelší strany v jednotkách mapy (výchozí 256), snap je mřížka pro rohy (0 pro žádnou) a slova `solid` nebo `layered` místo tenkých klínů vytvoří kvádry v mřížce nebo konvexní vrstvy.
- `--render-preview <in.map> <out.png> [size]`: zapíše náhled mapy jako PNG, ve výchozím stavu 512 pixelů.
- `--run-script <script.rhai> <in.map> [out.map] [key=value ...]`: spustí skript pluginu nad mapou se vším vybraným a zapíše výsledek (ve výchozím stavu zpět do vstupu). `key=value` odpovídá na nastavení pluginu.

# Řešení problémů
## Když něco nevypadá dobře
- Chybí materiál: zkontrolujte složku nebo balík materiálů a cesty projektu v exportu.
- Špatná velikost modelu: porovnejte se 64jednotkovou značkou hráče a měřítkem exportu.
- Špatný směr přechodu: zkontrolujte úhel přechodu a orientaci stěny.
- Prázdná scéna v enginu: přečtěte varování exportu a postupujte podle pokynů k importu pro daný cíl.
- Živé propojení čeká: ověřte, že je doplněk nainstalovaný, scéna připojená a aktualizace nejsou pozastavené.
- Jen čáry bez stěn: vypněte Drátový model v sekci Zobrazení na kartě Mapa, nebo v nabídce 3D okna zvolte Materiály či Plné plochy.
- V pohledech chybí objekty: `Ctrl+Shift+I` zobrazí skryté objekty; zkontrolujte vrstvy a filtry Zobrazení.
- Klávesa nic nedělá: nejprve klikněte do pohledu, aby měl klávesnici, a v Nastavení → Klávesnice zkontrolujte sadu zkratek.
- Úprava selhala: vraťte ji, zjednodušte geometrii a použijte **Zkontrolovat mapu** a **Zkontrolovat netěsnosti** v sekci **Problémy** na kartě Mapa.
- Mapa se neotevře ze správce souborů: mohla být předána už běžícímu editoru; hledejte jeho okno.

## Nastavení a zálohy
Nastavení, nedávné mapy, hvězdičky, nastavení exportu a pluginy jsou ve složce konfigurace: `~/.config/heron-brush` na Linuxu a macOS a `%LOCALAPPDATA%\heron-brush` na Windows. Zálohy automatického ukládání jsou ve složce `autosave` vedle každé mapy.

## Hlášení chyb
Uveďte verzi editoru (Nápověda → O programu), operační systém, malou mapu, na které je problém vidět, a přesný postup. Výstupní panel (`Ctrl+4`) často řekne, co se pokazilo. Před experimentováním si uložte kopii.

## Jak používat tuto příručku
Kapitolu zvolte v obsahu vlevo. `Ctrl+F` zobrazí obsah a přesune vás do pole hledání; `Esc` hledání vymaže, nebo příručku zavře, když je pole prázdné. **Obsah** skryje nebo zobrazí seznam kapitol a **Velikost písma** mění velikost textu; obě volby se pamatují. Seznam jazyků nahoře přepíná mezi dostupnými překlady.


Na desktopu Soubor → Sestavit a spustit ukládá pojmenované kroky export/kopie/spuštění vedle exportních nastavení projektu. Nastavení → Klávesnice → Upravit klávesové zkratky nabízí 30 příkazů a kontrolu konfliktů. Diagnostiky lze skrýt a neplatné UV nebo prázdný název vlastnosti opravit s Undo. Nápověda → Aktualizace aplikace kontroluje plánovaný repozitář MojzisekJan/HeronBrush; kontrola při spuštění je výchozí vypnutá. Podrobnosti proměnných, argumentů a omezení jsou v [dokumentaci build profilů a aktualizací](../build-and-updates.md).

Na desktopu Geometrie → Importovat model, nebo přetažení OBJ/GLB, vloží statickou modelovou entitu. Uložené mapy dostanou kopii assetů vedle mapy a Uložit jako je přenese do nové složky. V inspectoru entity upravte origin, angles a modelscale. Podporované formáty, exporty a omezení popisuje [Import statických modelů](../model-import.md).
