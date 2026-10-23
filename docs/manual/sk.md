# Začíname
Heron Brush je editor úrovní založený na brushoch. Úrovne skladáte z konvexných brushov, ich steny texturujete a farbíte, umiestňujete entity, animujete dvere a plošiny a výsledok exportujete do herného enginu. K dispozícii sú aj vstavané assety, parametrické tvary, živé prepojenie s editormi enginov, spoločné úpravy a vykresľovanie statických obrázkov. Táto príručka je súčasťou aplikácie a funguje bez pripojenia na internet.

Na macOS funguje `Cmd` všade, kde príručka uvádza `Ctrl`, a Option zodpovedá klávese Alt.

## Uvítacia obrazovka
Keď Heron Brush spustíte bez mapy, otvorí sa uvítacia obrazovka. Vľavo sú **Pokračovať v** (naposledy otvorená mapa), **Nová mapa** a **Prehľadávať** na otvorenie existujúceho súboru `.map`, ďalej **Hra**, **Príručka** a **Nastavenia**. Vpravo je zoznam nedávnych máp s náhľadmi a pod ním **Príklady**. Hviezdička (**Pripnúť navrch**) pripne mapu na začiatok zoznamu, kôš (**Presunúť do koša**) presunie súbor do systémového koša, odkiaľ ho možno obnoviť. Šípkami sa pohybujete po obrazovke a Enter otvorí zvolenú položku. Kedykoľvek sa k obrazovke vrátite cez Súbor → Vitajte.

## Prvá mapa
1. Na uvítacej obrazovke zvoľte **Nová mapa**, alebo Súbor → Nová mapa (`Ctrl+N`).
2. Na lište nástrojov vyberte nástroj Brush (`B`) a ťahaním v 2D pohľade nakreslite zvolený tvar. Hĺbku skontrolujte v inom 2D pohľade.
3. Brush vyberte a ťahaním ho presuňte. Na karte **Stena** mu priraďte materiál alebo farbu.
4. Zdrojovú mapu uložte pomocou `Ctrl+S`. Keď ju budete chcieť použiť v hre, otvorte Súbor → Export a zapíšte scénu pre svoj engine.

## Súbory a ukladanie
Súbor `.map` je váš upraviteľný zdroj. Ukladajte priebežne: export, živé prepojenie ani vykresľovanie mapu za vás neuložia. Súbor → Uložiť ako (`Ctrl+Shift+S`) doplní príponu `.map`, ak ju vynecháte. Súbor → Vrátiť sa k uloženej mape zahodí neuložené zmeny a načíta súbor znova. Ak zatvárate okno alebo otvárate inú mapu a máte neuložené zmeny, Heron Brush sa opýta, či **Uložiť**, **Zahodiť** alebo **Zrušiť**.

Kým má uložená mapa neuložené zmeny, každých desať minút sa do priečinka `autosave` vedľa mapy zapisujú číslované zálohy. Uchováva sa 50 najnovších záloh.

Ak otvoríte mapu, keď už Heron Brush beží, odovzdá sa bežiacemu oknu a to sa presunie do popredia. Ak chcete samostatné okno, použite na príkazovom riadku `--new-instance` (pozri Príkazový riadok).

# Rozhranie
Hlavné okno má titulkový pruh s ponukami, pod ním lištu nástrojov, pohľady na mapu, inšpektor vpravo a stavový riadok dole.

## Titulkový pruh a ponuky
Titulkový pruh obsahuje logo, ponuky, v strede názov mapy a tlačidlá okna. Na macOS sú obvyklé červené, žlté a zelené tlačidlá vľavo. Ťahaním za pruh okno presuniete, dvojklikom ho maximalizujete. Keď niečo beží na pozadí, objaví sa pri konci pruhu indikátor: živé prepojenie s enginom, živá relácia alebo **MCP aktívne**. Kliknutím na indikátor živé prepojenie zastavíte alebo otvoríte jeho okno.

- **Súbor**: Vitajte, Nová mapa (`Ctrl+N`), Otvoriť mapu (`Ctrl+O`), Uložiť (`Ctrl+S`), Uložiť ako (`Ctrl+Shift+S`), Vrátiť sa k uloženej mape, Export, Vykresliť obrázok, Pluginy a Nastavenia (`Ctrl+,`).
- **Úpravy**: Späť (`Ctrl+Z`), Znova (`Ctrl+Shift+Z`), Vystrihnúť, Kopírovať, Prilepiť, Duplikovať (`Ctrl+D`), Odstrániť, Vybrať všetko (`Ctrl+A`), Zopakovať posledné príkazy (`Ctrl+R`) a Zabudnúť opakovateľné príkazy.
- **Zobraziť**: 3D pohľad v samostatnom okne, Prejsť na, Prichytávať k mriežke (`Alt+0`), Zobraziť mriežku (`0`), Náhľad animácií a Zobraziť vynechané steny.
- **Geometria**: Vložiť 3D text, Spoj v rohu, Previesť steny na patche, Vyňať výber z jeho entity a Zosúladiť prepojené skupiny.
- **Animácia**: Animácia kľúčovými snímkami, Náhľad animácií a Umiestniť kameru podľa 3D pohľadu (`Ctrl+Alt+0`).
- **Mapa**: načítanie, opätovné načítanie a zatvorenie súboru bodov a súboru portálov. Ukazujú stopy netesností a portály, ktoré zapisujú kompilátory máp.
- **Spolupráca**: Živá relácia a Sprístupniť agentovi (MCP).
- **Pomocník**: Príručka a O programe.

## Lišta nástrojov
Lišta nástrojov nesie všetky nástroje. Prvú skupinu používate neustále: Vybrať, Brush, Rez, Vrchol, Hrana, Stena, Otočiť a Mierka. V druhej skupine sú Skosiť, Skladať, Pretiahnutie, Riadiaci bod, Zaoblenie a Maľovanie. Po nabehnutí myšou na tlačidlo sa zobrazí jeho názov a skratka. Opätovné kliknutie na aktívny nástroj vráti nástroj Vybrať.

Za nástrojmi nasledujú **Prevrátiť vodorovne** a **Prevrátiť zvisle** a potom dva prepínače: **Zamknúť** zachová mapovanie materiálu pri posune brushu a **UV** pri jeho otočení (prepína ho `U`). Potom prídu nastavenia aktívneho nástroja: nástroj Brush ukazuje tvar a jeho voľby, nástroj Zaoblenie polomer, počet stien, rohy a tlačidlo **Použiť**. Na pravom konci je názov nástroja, veľkosť mriežky (**Mriežka 16** a pod.; kliknutím zvolíte inú) a ponuka **Nastavenia mriežky** s položkami Prichytávať k mriežke, Zobraziť mriežku, Náhľad animácií, Zobraziť vynechané steny a Animácia kľúčovými snímkami.

## Pohľady na mapu
K dispozícii sú štyri pohľady: 3D pohľad a tri ploché pohľady Zhora (XY), Spredu (XZ) a Zboku (YZ). Nastavenia → Zobraziť → Rozloženie ponúka Jeden panel, Dva panely, Tri panely alebo Štyri panely. Ťahaním priečok meníte veľkosť panelov. Kliknutím do pohľadu mu odovzdáte klávesnicu. `Space` prepne panel, ktorý ukazuje jeden pohľad naraz, na ďalší pohľad, a `Ctrl+Space` maximalizuje pohľad s klávesnicou a znova obnoví rozloženie.

Kliknutím pravým tlačidlom bez ťahania v ľubovoľnom pohľade otvoríte miestnu ponuku pre výber a to, čo je pod ukazovateľom. Ponúka zoskupovanie, vrstvy, prepojené skupiny, vytváranie entít, presun brushov do entity a z nej, kopírovanie názvu materiálu a jeho zobrazenie v prehliadači materiálov.

## Inšpektor
Inšpektor vpravo má štyri karty. Jeho šírku zmeníte ťahaním ľavého okraja.

- **Mapa** (`Ctrl+1`): Vrstvy, Skupiny, Pluginy, Problémy, filtre zobrazenia a Značky.
- **Entita** (`Ctrl+2`): prehliadač entít, definície entít, vlastnosti vybranej entity a pri tlačidlách a spúšťačoch Spúšťač animácie.
- **Stena** (`Ctrl+3`): UV, Materiály, Farba, Pretiahnutie, Zaoblenie, Patch a Zapustenie a skosenie. Každú sekciu možno zbaliť.
- **Assety**: knižnica opakovane použiteľných modelov z brushov.

`Ctrl+5` skryje alebo zobrazí inšpektor a `Ctrl+Alt+T` skryje alebo zobrazí lištu nástrojov, takže pohľady získajú viac miesta.

## Stavový riadok a výstupný panel
Stavový riadok dole ukazuje poslednú správu editora. Chyby sú červené. Kliknutím na riadok alebo klávesmi `Ctrl+4` otvoríte výstupný panel s celou históriou správ. Väčšina príkazov sem hlási, čo urobila, napríklad koľko brushov rozrezala.

## Dialógy a vzhľad
Dialógy ako Nastavenia, Export a Vykresliť obrázok sa otvárajú vo vlastných oknách. Nastavenia → Zobraziť → Farebná schéma ponúka Podľa systému, Svetlá alebo Tmavá. **Krytie panelov (%)** na stránke Prehľad spriehľadní panely nad pohľadmi. Okno kreslí vlastné tlačidlá v štýle systému: natívne na macOS, v štýle Windows 11 na Windows a v štýle GNOME na Linuxe.

# Ovládanie a klávesnica
## Pohyb v 3D pohľade
Držte pravé tlačidlo myši a ťahaním sa rozhliadajte. Ťahaním stredným tlačidlom posúvate pohľad. Koliesko pohybuje dopredu a dozadu. `W`, `A`, `S` a `D` letia dopredu, doľava, dozadu a doprava, `Q` stúpa a `X` klesá; so `Shift` sú kroky väčšie. Ak je v Nastavenia → Myš zapnuté **Alt + ťahanie stredným tlačidlom posúva kameru**, Alt so stredným tlačidlom posúva kameru v smere pohľadu. Rýchlosti, prevrátenie osí a zorný uhol nájdete v Nastavenia → Myš.

## Pohyb v 2D pohľadoch
Ťahaním stredným tlačidlom posúvate pohľad a koliesko približuje okolo ukazovateľa. Posun a priblíženie jedného plochého pohľadu udrží ostatné zarovnané; riadi to **Prepojiť 2D pohľady** v Nastavenia → Myš. `Ctrl+Alt+Z` obnoví priblíženie a `Ctrl+Shift+U` vráti ploché pohľady na počiatok.

`Ctrl+U` zarámuje výber vo všetkých pohľadoch. Zobraziť → Prejsť na (`Ctrl+Alt+M`) presunie kameru na zadané súradnice, vyberie objekt na danom riadku súboru mapy alebo posunie výber o zadanú vzdialenosť.

## Mriežka a prichytávanie
`+` a `-` menia veľkosť mriežky a číselné klávesy `1` až `9` volia veľkosť priamo, od 1 do 256 jednotiek. `0` alebo `#` zobrazí či skryje mriežku a `Alt+0` zapne alebo vypne prichytávanie. `S` prichytí vrcholy vybraných brushov k aktuálnej mriežke, `Ctrl+Shift+V` k celým jednotkám a `Ctrl+Alt+Shift+V` k mriežke.

## Bežné skratky
- `F3` alebo `Ctrl+Shift+P`: vyhľadá ľubovoľný príkaz z menu podľa názvu a `Enter` ho spustí. `F10` otvorí menu a šípky sa v nich pohybujú.
- `Ctrl+S`: uložiť mapu. `Ctrl+Shift+S`: uložiť ako.
- `Ctrl+Z`: späť. `Ctrl+Shift+Z` alebo `Ctrl+Y`: znova.
- `Ctrl+C`, `Ctrl+X`, `Ctrl+V`: kopírovať, vystrihnúť a prilepiť. Prilepenie umiestni objekty k ukazovateľu; `Ctrl+Alt+V` zachová ich pôvodné súradnice. Ak sú vybrané steny, kopíruje sa ich materiál a nastavenie UV.
- `Ctrl+D`: duplikovať. `Ctrl+Shift+D`: prepojená kópia skupiny.
- `Delete` alebo `Backspace`: odstrániť výber.
- `Esc`: zrušiť prebiehajúce ťahanie, vrátiť krok v aktívnom nástroji, prepnúť na Vybrať a nakoniec zrušiť výber. `Shift+Esc` prepne rovno na Vybrať.
- `Ctrl+R`: zopakovať posledné opakovateľné príkazy. `Ctrl+Shift+R`: zabudnúť ich.
- `Ctrl+Alt+0` alebo `Ctrl+Alt+Num0`: umiestniť renderovaciu kameru podľa 3D pohľadu.
- `F5`: znova načítať materiály. `F6`: znova načítať definície entít.

## Skratky pre výber
- `Ctrl+A`: vybrať všetko, alebo všetky úchyty, keď je aktívny nástroj Vrchol, Hrana alebo Stena. `Ctrl+Shift+A`: zrušiť výber. `Ctrl+Alt+A`: obrátiť výber.
- `Ctrl+B`: vybrať ostatné objekty v tej istej entite alebo skupine.
- `Ctrl+T`: vybrať všetko, čoho sa vybrané brushe dotýkajú. `Ctrl+E`: vybrať všetko vo vnútri nich. `Ctrl+Shift+E`: vybrať všetko v ich stĺpci pozdĺž osi pohľadu. Výberové brushe sa pritom spotrebujú.
- `Ctrl+I`: izolovať výber. `Ctrl+Alt+I`: skryť ho. `Ctrl+Shift+I`: znova zobraziť všetko.
- `Ctrl+G`: zoskupiť. `Ctrl+Shift+G`: zrušiť zoskupenie.

## Skratky pre transformácie
- Šípky a `Page Up` / `Page Down` posunú výber o jeden krok mriežky v smere, ktorým ukazujú v aktívnom pohľade. S `Ctrl` pri posune duplikujú, s `Alt` otáčajú.
- `Alt+X`, `Alt+Y`, `Alt+Z`: otočiť o štvrť otáčky okolo danej osi; so `Shift` opačným smerom.
- `Ctrl+F`: prevrátiť naprieč pohľadom; `Ctrl+Alt+F`: prevrátiť hore a dole. `Ctrl+Shift+X` a `Ctrl+Shift+Y` zrkadlia pozdĺž X alebo Y.
- `Insert`: umiestniť bodovú entitu do stredu výberu.
- `Tab`: s vybraným brushom prepína Vybrať → Vrchol → Hrana → Stena → Vybrať.

## Klávesové skratky Blender
Nastavenia → Klávesnica → Klávesové skratky ponúka Heron Brush (predvolené, používa ich celá príručka) a Blender. So skratkami Blender: `G` vyberá a posúva, `R` otáča, `S` mení mierku, `E` preťahuje, `1`, `2` a `3` volia nástroje Vrchol, Hrana a Stena, `K` reže, `B` vyberá, `A` vyberie všetko a `Alt+A` výber zruší, `X` odstraňuje, `Shift+D` duplikuje, `H` skryje a `Alt+H` všetko zobrazí a `Ctrl+Shift+Z` vykoná znova. Všetko ostatné je v oboch sadách rovnaké.

# Brushe a modelovacie nástroje
Brushe sú konvexné telesá definované rovinami. Zložité objekty skladajte z niekoľkých jednoduchých brushov, nesnažte sa urobiť jeden brush nekonvexný. Kde stačí jednoduchý tvar, držte počet brushov nízko.

## Kreslenie brushov
Zvoľte nástroj Brush (`B`) a ťahajte v pohľade. Lišta nástrojov ukazuje **Kreslený tvar**: Kváder, Schodisko, Klenba, Valec, Kužeľ, Sféroid (UV), Sféroid (dvadsaťsten) a Vložiť 3D text. `Alt+1` až `Alt+7` volia tvar priamo. Podľa tvaru lišta ukazuje **Dutý** a **Hrúbka:**, pri schodoch **Výška schodu:** a **Smer:**, pri okrúhlych tvaroch **Os:**, **Strany:** a režim kružnice, pri guliach **Prstence:** a pri dvadsaťstene **Presnosť:**. Voľby nastavte pred ťahaním. Výsledok skontrolujte vo viacerých pohľadoch.

## Výber
Kliknutím brush vyberiete, `Ctrl`-kliknutím ho pridáte do výberu. Ťahanie z prázdneho miesta kreslí výberový obdĺžnik. V 3D pohľade `Shift`-kliknutie vyberie jednu stenu a `Ctrl+Shift`-kliknutie pridá ďalšie steny. Ťahaním vybraného brushu ho presuniete; ťahaním nevybraného vyberiete a presuniete len ten.

V 3D pohľade `Shift`-ťahaním steny vybraného brushu stenu vytiahnete. `Alt`-kliknutím na inú stenu na ňu prenesiete materiál a zarovnanie vybranej steny; `Alt+Ctrl` prenesie len materiál a `Alt+Shift` otočenie.

## Tvarovacie nástroje
- **Vybrať** (`Esc`): výber a posun.
- **Vrchol** (`V`), **Hrana** (`E`), **Stena** (`F`): ťahanie rohov, hrán alebo stien. `Shift`-ťahanie zo stredu hrany alebo steny tam pridá roh. `Delete` odoberie vybrané rohy.
- **Otočiť** (`R`), **Mierka** (`T`), **Skosiť** (`G`): transformácia výberu pomocou úchytov.
- **Rez** (`C`): dvoma alebo troma kliknutiami určte reznú rovinu. `Enter` reže, `Ctrl+Enter` volí, ktorá strana zostane, a `Backspace` alebo `Esc` odoberie posledný bod.
- **Skladať** (`N`): rozmiestnite body okolo objemu a `Enter` z nich vytvorí konvexný brush. `Backspace` odoberie posledný bod.
- **Pretiahnutie** (`Y`): pozri sekciu o pretiahnutí nižšie.
- **Riadiaci bod** (`P`): úprava riadiacej mriežky patchu.
- **Zaoblenie** (`O`): kliknutím na hranu ju zaoblíte, alebo vyberte hrany či vrcholy ich nástrojmi a stlačte `Enter`. Viac stien dáva hladší výsledok a viac geometrie.
- **Maľovanie** (`K`): pozri Materiály a UV.

## Kombinovanie brushov
- `Ctrl+J`: zlúčiť vybrané brushe do ich konvexného obalu.
- `Ctrl+K`: odčítať vybrané brushe od brushov, ktorých sa dotýkajú.
- `Ctrl+Shift+K`: vydutiť výber so stenami hrubými jeden krok mriežky.
- `Ctrl+L`: prienik výberu.

## Pretiahnutie, zaoblenie, patch a skosenie
**Pretiahnutie** vytvára geometriu z vybraných stien pozdĺž dráhy. Vyberte steny (v nástroji Pretiahnutie `Alt`-kliknutím), v sekcii Pretiahnutie na karte Stena nastavte **Dráha**, **Otočenie** (os a uhol), **Mierka**, **Kroky** a **Cesta** (zarovnanie Celé číslo alebo Voľné; Oblúk, Rovná alebo Esovitý ohyb) a stlačte **Pretiahnutie**. Riadok pod ovládacími prvkami hovorí, koľko brushov vznikne.

Sekcia **Zaoblenie** zaoblí vybrané hrany podľa hodnôt **Polomer**, **Steny** a **Rohy** (Zrezané alebo Zaoblené); jedna stena znamená zrazenie hrany.

**Patch** ukazuje **Stĺpce** a **Riadky** vybraného patchu s tlačidlami na pridanie alebo odobratie dvojice. Geometria → Previesť steny na patche (`Ctrl+P`) vytvorí patche z vybraných stien.

**Zapustenie a skosenie** pracuje s vybranými stenami: **Zapustenie** stiahne stenu od jej hrán dovnútra a **Hĺbka** ju vysunie von (alebo dnu, ak je záporná). **Skosiť steny** použije oboje; tretie tlačidlo pridá panel alebo vyreže výklenok podľa znamienka hĺbky. **Spoj v rohu** otvorí dialóg, ktorý spojí dva brushe zrezaným alebo zaobleným rohom s náhľadom.

## 3D text
Geometria → Vložiť 3D text (aj posledná položka zoznamu tvarov nástroja Brush) vytvorí brushe z textu. Používa Heron Block, vstavané polygónové písmo so zrazenými rohmi, proporcionálnymi šírkami a českou diakritikou; malé písmená majú tvar veľkých. Napíšte **Text**, nastavte **Výška** (výška veľkých písmen; diakritika siaha vyššie) a **Hĺbka**, sledujte živý náhľad a stlačte **Vložiť**.

Ak operácia nemôže vytvoriť platnú geometriu, upravte rozmery, alebo zmenu vráťte a tvar zjednodušte.

## Pole
Geometria → Pole skopíruje výber a kópie zobrazuje v mape, kým meníte hodnoty. Počty zahŕňajú originál a pole vytvorí najviac 1024 kópií.

- **Rad** posunie každú kópiu oproti predchádzajúcej o **Krok X**, **Krok Y** a **Krok Z** a otočí ju o **Otočenie na kópiu (stupne)** okolo zvolenej osi.
- **Mriežka** vyplní **Počet X**, **Počet Y** a **Počet Z** položiek s rozstupmi **Krok X**, **Krok Y** a **Krok Z**.
- **Kruh** rozmiestni položky v rozsahu **Uhol (stupne)** okolo stredu vzdialeného **Polomer (jednotky)** od výberu. So zapnutým **Natočiť k stredu** sa každá kópia otáča s kruhom.
- **Rozptyl** umiestni kópie na náhodné miesta v danom polomere naprieč zvolenou osou. Rovnaký **Seed** dá rovnaké rozmiestnenie a **Náhodné otočenie** každú kópiu otočí.

Keď je vybraná jedna celá skupina, **Prepojené kópie** urobí z každej kópie prepojený duplikát, takže úprava jednej sa prejaví vo všetkých. **Použiť** kópie ponechá a vyberie ich spolu s originálom; **Zrušiť** ich odstráni. Celé pole sa vracia jedným krokom späť.

## Sochanie terénu
Terén tvorí veľa malých brushov, ktorých povrch sochársky štetec vytláča a vtláča. Vyberte jeden alebo viac brushov a zvoľte Geometria → Rozdeliť na stĺpce a potom tvar stĺpcov: Trojuholník, Štvorec alebo Šesťuholník. Rozreže ich pozdĺž čiar mriežky na stĺpce široké jeden krok mriežky a vloží ich do skupiny Terrain. Rezy idú po mriežke sveta, takže brushe vedľa seba sa rozrežú na rovnakých miestach a zdieľajú rohy. Každý stĺpec stojí pozdĺž najtenšej strany brushu: zvislo v podlahe alebo strope, vodorovne v stene. Rovnako vznikne jaskyňa: z kvádra urobte dutinu (Hollow selection) s hrúbkou steny jeden krok mriežky, vyberte steny a rozdeľte ich.

Výber sa dá rozdeliť aj v okne **Sochať**, ktoré otvorí tlačidlo na konci lišty sochania, a to na tvar zvolený v jeho riadku Tvar stĺpcov. Obe miesta si pamätajú naposledy použitý tvar. **Trojuholník** dáva dva stĺpce na bunku mriežky a vrch trojuholníka zostane rovný, nech sa tvaruje akokoľvek. **Štvorec** dáva jeden stĺpec na bunku, teda polovičný počet brushov; jeho vrch sa ohne pozdĺž uhlopriečky. **Šesťuholník** pokryje brush šesťuholníkmi širokými dva kroky mriežky. Každý sa rozdelí na šesť trojuholníkov okolo stredu, takže všetky vrchy zostanú rovné.

Zvoľte nástroj **Sochať** (`J`) a ťahajte po povrchu v 3D pohľade. Keď tlačidlo držíte na mieste, štetec ďalej pracuje na rovnakom mieste, desaťkrát za sekundu, takže kopec rastie, kým držíte. Oranžový kruh ukazuje štetec. Bez výberu štetec mení všetky brushe pod sebou, s výberom len tie vybrané. Lišta hore v okne určuje, čo ťah urobí:

- **Zdvihnúť** a **Znížiť** posunú povrch von alebo dovnútra, najviac v strede kruhu a na okraji vôbec. Podržaním `Shift` ich prehodíte.
- **Vyhladiť** zarovná hrbolčeky.
- **Zarovnať** zarovná povrch tam, kde ťah začal.

V tom istom okne **Smer** určuje kadiaľ:

- **Hore** posúva povrch zvislo, v kruhu na zemi. To je pre terén.
- **Kamera** ho posúva k pohľadu a od neho, v guli okolo ukazovateľa. Pozrite sa na podlahu, stenu alebo strop a tvaruje sa to, na čo sa pozeráte.
- **Normála** ho posúva von zo steny, nad ktorou ťah začal.

Dve čísla sú **Polomer (jednotky)** a **Sila (%)**. Posúvajú sa len rohy na strane, ktorou ťah hýbe; zadná strana zostáva na mieste a stĺpec zostane aspoň jednu jednotku hrubý. Roh, ktorý brushe zdieľajú, sa posunie vo všetkých rovnako, takže povrch zostane uzavretý. Ťah sa vracia jedným krokom späť. V 2D pohľadoch nástroj vyberá rovnako ako nástroj Výber.

Keď budete so sochaním hotoví, vyberte terén a zvoľte Geometria → **Zlúčiť stĺpce**, alebo tlačidlo v okne **Sochať**. Susedné stĺpce, ktoré spolu tvoria konvexný brush a vyzerajú rovnako, sa zlúčia do jedného, takže z rovných plôch zostane pár veľkých brushov. Povrch sa nepohne a zlúčenie sa vracia jedným krokom späť.

**Zlúčiť so zachovaním tvaru**, hneď vedľa v ponuke aj v okne, zlučuje rovnako, ale nehľadí na vzhľad stien: kde sa dve steny spoja do jednej, dostane materiál jednej z nich. Funguje na akýchkoľvek vybraných brushoch, nielen na teréne. Štyri kvádre do L sa zlúčia do dvoch, nikdy do jedného, ktorý by L vyplnil, a celý vytvarovaný terén klesne na toľko brushov, koľko zlučovanie susedov nájde. Zlučujú sa len brushe v rovnakej skupine alebo entite, takže dvere nikdy nepohltia múr vedľa seba.

Pre terén ako vo voxelových hrách rozdeľte na stĺpce **Štvorec** a v okne **Sochať** zvoľte pri **Ťah** možnosť **Kocky**. Ťah potom posúva celý vrch každého stĺpca po celých krokoch mriežky a boky necháva zvislé, takže terén stúpa v schodoch. Všetky režimy fungujú ako predtým; malé ťahy sa sčítavajú, kým stĺpec nevystúpi o celý krok. Stĺpce iných tvarov zostanú, ako sú.

**Voľne** nepotrebuje stĺpce, funguje na akýchkoľvek brushoch. Zdvihnúť pridáva hmotu v guli okolo ukazovateľa a Znížiť ju uberá, takže terén môže tvoriť previsy a jaskyne. Vyhladiť ho vyrovná a Zarovnať ho zrovná do roviny, kde ťah začal. Každý dab prestavá zasiahnuté brushe na mriežke so zaoblením a všetko mimo gule nechá, ako bolo, vrátane ostrých hrán. Výsledkom sú obyčajné brushe, takže na ne fungujú všetky ostatné nástroje, a **Zlúčiť stĺpce** potom zníži ich počet. Jemnejšia mriežka dáva hladší povrch a viac brushov.

# Materiály a UV
## Načítanie a použitie materiálov
Otvorte sekciu **Materiály** na karte Stena. **Načítať priečinok** pridá priečinok obrázkov a **Načítať .wad** balík textúr; oba sa objavia v **Kolekcie**, kde ich krížik odoberie. Písaním do **Filtrovať materiály** zúžite zoznam. Kliknutím zvolíte aktuálny materiál a **Použiť na výber** ho dá na všetky vybrané steny. **Vybrať** ponúka Vybrať brushe s ním a Vybrať steny s ním. **Nahradiť** zamení jeden materiál za iný v celej mape alebo len vo výbere.

Nástroj **Maľovanie** (`K`) dá aktuálny materiál na stenu pod ukazovateľom: kliknutie namaľuje jednu stenu, dvojklik celý brush a `Shift`-kliknutie prevezme materiál steny ako aktuálny, podobne ako kvapkadlo.

## Editor UV
Sekcia **UV** na karte Stena ukazuje materiál a obrys vybranej steny. Ovládanie myšou na plátne:
- Koliesko približuje okolo ukazovateľa. Ťahaním pravým alebo stredným tlačidlom posúvate. Dvojklikom pravým tlačidlom stenu znova zarámujete; výber inej steny ju zarámuje automaticky.
- Ťahanie ľavým tlačidlom posúva materiál, obrys steny zostáva na mieste. Kruh otáča mapovaním, rohové úchyty menia mierku a stredový úchyt určuje počiatok týchto zmien.
- `Alt`-ťahanie ľavým tlačidlom za okraj dlaždice skosí, rovnako ako `Shift`-ťahanie stredným tlačidlom.
- Každé ťahanie je jeden krok späť a `Esc` prebiehajúce ťahanie zruší.

Pod plátnom sú **Posun**, **Mierka**, **Uhol** a, ak ich hra používa, **Príznaky obsahu**, **Príznaky povrchu** a **Hodnota povrchu**. Tlačidlá sú **Obnoviť**, **Zarovnať podľa sveta**, **Prispôsobiť**, **Prevrátiť vodorovne**, **Prevrátiť zvisle** a dve otočenia o 90°. **Zarovnať** natočí materiál pozdĺž jednej z hrán steny (ďalším stlačením ďalšia hrana, so `Shift` predchádzajúca). Štyri tlačidlá zarovnania prisunú materiál k hrane. **Vyplniť U** a **Vyplniť V** zmenia mierku tak, aby stenu pokryl celý počet opakovaní; ďalším stlačením ďalší počet, s `Ctrl` sa zachová delenie trim sheetu.

## Klávesy pre UV
S vybranými stenami šípky posúvajú materiál, `[` a `]` ho otáčajú o 15°, `,` a `.` jeho mierku zmenšia na polovicu alebo zdvojnásobia, `Ctrl+F` a `Ctrl+Alt+F` ho prevrátia, `Shift+R` ho obnoví, `Alt+Shift+R` ho zarovná podľa osí sveta a `Ctrl+Shift+F` ho prispôsobí každej stene.

**Zamknúť** na lište nástrojov zachová mapovanie pri posune brushov a **UV** (`U`) pri ich otáčaní.

## Materiály nástrojov
- skip: vyradí označenú stenu z exportu do enginov.
- clip: vytvorí kolíziu bez viditeľnej geometrie.
- origin: označí pivot brushovej entity.

Názvy možno zmeniť v karte **Materiály nástrojov** v okne Export. Skip používajte pre steny, o ktorých viete, že sú skryté.

## Vynechanie stien z exportu
Vyberte stenu a v sekcii UV zapnite **Vynechať stenu z exportu**. Oranžové šrafovanie ju označí v náhľade UV. Jej materiál aj farba zostanú uložené a brush zostane uzavretý a upraviteľný. Exportované vykresľovacie siete stenu vynechajú, kolízia zostane. Ak z kocky ponecháte jeden štvoruholník, vzniknú dva vykresľovacie trojuholníky. Voľba nerobí steny obojstrannými ani nezlučuje siete.

**Vynechať zakryté steny** označí steny úplne zakryté statickými brushmi, vrátane zhodných dotýkajúcich sa stien a uzavretých brushov. Pracuje s vybranými brushmi, alebo so všetkou viditeľnou a nezamknutou geometriou sveta, keď nie je nič vybrané. Brushové entity, priehľadné materiály a brushe, ktoré už vynechané steny majú, sa preskočia. Čiastočne zakryté steny sa nedelia. Jedno vrátenie obnoví všetko. Zobraziť → Zobraziť vynechané steny také steny v pohľadoch editora skryje alebo ukáže; výber stále pracuje s celým brushom.

# Farby a prechody
Sekcia **Farba** na karte Stena farbí steny jednoliatymi farbami alebo prechodmi navrch ich materiálov. Hodí sa na hrubý návrh úrovne aj na štylizovanú grafiku. Zafarbené steny si v pohľadoch ponechajú skutočné farby; výber je vyznačený obrysom.

## Paleta a miešaná farba
Kliknutím na jednu z dvanástich vzoriek palety ňou zafarbíte vybrané steny alebo celé vybrané brushe. **Krytie** nastavuje v percentách, aká krycia bude ďalšia farba. **Miešať** má farebné tlačidlo, ktoré otvorí výber farby; pri ťahaní vo výbere farby sa výber mení naživo, alebo tlačidlom **Maľovanie** použijete miešanú farbu ako jeden krok. **Späť na materiál** farbu odstráni a stena znova ukáže svoj materiál. Riadok navrchu sekcie hovorí, čo je vybrané a čím je to zafarbené.

## Výber farby
Každé farebné tlačidlo v editore otvára rovnaký výber farby. Ťahaním v štvorci volíte sýtosť a jas, v pruhu vedľa neho odtieň. Pod nimi sú hodnoty **R**, **G** a **B** a posuvník **Krytie**, ak farba krytie má. Pole s hexadecimálnym kódom prijíma `#RRGGBB` alebo `#RRGGBBAA`; kým text nie je platná farba, je červené. **Kopírovať** vloží kód do schránky. Kliknutím mimo výberu farby ho zatvoríte.

## Prechody
**Prechod** volí Lineárny prechod alebo Radiálny prechod a tlačidlo vedľa neho vymení farby koncov. Každý bod má farebné tlačidlo a **Poloha (%)**, body medzi koncami majú aj tlačidlo na odobratie. **Pridať farebný bod** pridá bod do najväčšej medzery; prechod môže mať až osem bodov. **Uhol prechodu** určuje smer lineárneho prechodu, s tlačidlami **+45°** a **−45°**.

Pod náhľadom je lišta so značkou pre každý bod. Kliknutím na lištu pridáte bod, kliknutím na značku otvoríte jej výber farby, ťahaním značku posuniete a kliknutím pravým tlačidlom ju odoberiete. Tri vzorky pod lištou sú predvoľby: Teplý prechod, Studený prechod a Prechod v odtieňoch sivej. Zmeny prechodu sa na výber prejavia naživo.

Na smere prechodu záleží: veďte ho pozdĺž sklonu strechy alebo zakriveného objektu, nie naprieč nesúvisiacimi stenami. Po zafarbení model skontrolujte z oboch strán.

## Materiál povrchu
**Materiál povrchu** nastavuje **Drsnosť** (0 je lesklý, 100 matný), **Kovovosť** (0 pre drevo, kameň či plast, 100 pre kov) a **Emisia** (0 až 8; povrch žiari vlastnou farbou, vrátane prechodov, ale okolité objekty neosvetľuje). Zmeny sa na vybrané steny alebo brushe použijú naživo. **Použiť predvolený povrch** prepísanie odstráni. Všetky zmeny možno vrátiť.

Efekty povrchu vidno v osvetlenom 3D pohľade a vo vykreslení kamerou. Ploché a drôtové zobrazenie odlesky zámerne vynecháva. Bloom môže okolo žiariacich povrchov pridať žiaru. Godot a glTF/GLB dostanú hodnoty materiálu a importér pre Unity prevedie drsnosť na hladkosť. Obmedzenia jednotlivých cieľov opisuje `docs/surface-materials.md`.

# Assety, vrstvy a skupiny
## Knižnica assetov
Otvorte kartu **Assety**. Priečinok modelov `.map` pridáte tlačidlom **Pridať priečinok**; vstavaná knižnica je k dispozícii vždy. Písaním do **Filtrovať assety** vyhľadávate. Kliknutím model vyberiete, dvojklikom, klávesom `Enter` alebo tlačidlom **Vložiť** ho pridáte tam, kde bol naposledy ukazovateľ. Model môžete aj pretiahnuť do pohľadu. Brushe assetu sa vkladajú ako jedna skupina, takže sa model pohybuje ako celok.

**Uložiť výber ako asset** zapíše vybrané brushe do knižnice ako `.map`. Assety v knižnici sú bežné upraviteľné brushe, nie importované siete.

## Vrstvy
Zoznam **Vrstvy** na karte Mapa organizuje veľké oblasti. Napíšte názov a kliknutím na **Pridať** vytvorte vrstvu, alebo klávesom `Enter` premenujte aktuálnu. Každú vrstvu možno skryť, zamknúť alebo vynechať z exportu. **Presunúť sem** presunie výber do aktuálnej vrstvy. Miestna ponuka v pohľadoch ponúka aj Presunúť do vrstvy, Nastaviť ako aktívnu vrstvu, Skryť vrstvy a Izolovať vrstvy.

## Skupiny a prepojené skupiny
`Ctrl+G` výber zoskupí a `Ctrl+Shift+G` zoskupenie zruší. Sekcia **Skupiny** na karte Mapa skupiny vypisuje a ponúka **Otvoriť**, **Premenovať**, **Zlúčiť**, **Zrušiť zoskupenie**, **Vyňať** a **Vyčleniť**. Prepojená skupina je sada kópií, ktoré zdieľajú úpravy: `Ctrl+Shift+D` vytvorí prepojenú kópiu, Geometria → Zosúladiť prepojené skupiny kópie aktualizuje a **Zrušiť prepojenie** urobí z kópie samostatnú skupinu. Na karte Entita možno vlastnosť ponechať ako vlastnú jednej kópii, keď sa prepojené skupiny zosúlaďujú.

## Spoločná mierka
Predvolená mierka exportu je 32 jednotiek mapy na meter. Vstavaná značka štartu hráča je vysoká 64 jednotiek, teda 2 metre. Porovnávajte s ňou dvere, vozidlá, nábytok aj rastliny. Mierku exportu meňte len zámerne a súvisiace assety udržujte konzistentné.

# Entity a hrateľnosť
Bežné definície entít sú vstavané a vždy k dispozícii: svetlá, štarty hráča, dvere, tlačidlá, plošiny, spúšťače, statické rekvizity, zvuky a body trasy. Načítanie FGD projektu cez **Definície entít** na karte Entita nahradí triedy s rovnakým názvom a ostatné vstavané ponechá.

## Umiestňovanie entít
V prehliadači **Entity** na karte Entita napíšte do **Filtrovať triedy** a zvoľte triedu. Bodovú triedu, napríklad svetlo, umiestnite tlačidlom **Umiestniť** alebo dvojklikom na triedu; `Insert` jednu umiestni do stredu výberu. Pri brushovej triede, napríklad dverách alebo spúšťači, vyberte brushe a kliknite na **Vytvoriť**. Triedy možno aj pretiahnuť do pohľadu.

## Vlastnosti
Zoznam vlastností upravuje vybranú entitu. Entity rovnakej triedy sa upravujú spoločne. **Pridať vlastnosť** pridá kľúč, ktorý definícia nemusí poznať; **Predvolené** vie vrátiť existujúce vlastnosti na predvolené hodnoty alebo doplniť chýbajúce. Vlastnosti, ktoré trieda definuje, ale entita ich nemá, sú zobrazené stlmene; zadaním hodnoty sa pridajú. Na prepojenie entít použite targetname a target.

## Čo dokáže export
Direktívy pre Godot vytvoria natívne uzly, kolízne telesá a oblasti spúšťačov. Unity a Unreal podporujú natívne bežné svetlá a Unreal navyše vytvára aktérov PlayerStart. Ostatné údaje sa zachovajú, nakoľko to importér cieľa dovolí.

Samotná entita dverí nepridá herné správanie. Pohyb, reakcie spúšťačov, ovládanie hráča a skripty projektu treba stále urobiť v engine. Defold exportuje vlastnosti entít a kolízne objekty; Flax umiestňuje modely a všeobecné entity. Kým na výsledku postavíte hrateľnosť, skontrolujte ho v engine.

# Animácia
Heron Brush dokáže zobraziť náhľad jednoduchého pohybu, upravovať animácie brushových entít kľúčovými snímkami a exportovať ich do viacerých enginov.

## Náhľad animácií
Zapnite **Náhľad animácií** v ponuke Zobraziť, v ponuke Animácia, v ponuke Nastavenia mriežky alebo v sekcii Zobraziť na karte Mapa. Prehráva rotáciu func_rotate a pohyb tam a späť func_shuttle, keď je zapnuté start_on, podľa vlastností osi, rýchlosti, smeru, dráhy a čakania, a prehráva klipy kľúčových snímok nastavené na Prehrať pri štarte. Assety ventilátora a veterného mlyna už otáčajúce sa časti obsahujú.

Vypnutie náhľadu obnoví pôvodnú pózu. Kliknutie do pohľadu zastaví prehrávanie ešte pred výberom alebo úpravou; ovládanie kamery funguje ďalej. Náhľad nemení mapu, históriu úprav ani exporty a nespúšťa fyziku ani hernú logiku.

## Editor kľúčových snímok
Otvorte ho cez Animácia → Animácia kľúčovými snímkami alebo z ponuky Nastavenia mriežky. Ukotví sa pod pohľadmi, alebo v 3D okne, kým je samostatné. Ikona okna v jeho záhlaví ho presunie do vlastného okna; snímka aj história úprav zostanú. **Upraviť animáciu** prepne pohľady na animovanú pózu a **Upraviť mapu** sa vráti ku kľudovej geometrii. V režime animácie každý pohľad dole ukazuje **Upraviť animáciu** a číslo snímky.

Animovať môžete vybranú brushovú entitu, brush alebo skupinu brushov. Plochá skupina brushov zdieľa jednu animáciu a pivot; skupiny s viacerými entitami potrebujú samostatné animácie.

## Klipy
Výber **Animácia** prepína medzi pomenovanými klipmi. Napíšte názov do **Názov animácie** a tlačidlom **Uložiť ako** skopírujete aktuálny klip pod týmto názvom; **Odstrániť** klip odstráni. **Posunúť** vytvorí klip `translate` bez rotácie. **Animácie dverí** vytvorí plynulé klipy `open` a `close` s otočením o 90 stupňov okolo aktuálneho pivotu; pivot najprv presuňte do závesu. **Prehrať pri štarte** spustí klip pri štarte úrovne; predvoľba dverí túto voľbu vypína.

## Ovládanie prehrávania
Tlačidlá prehrávania sú **Začiatok**, **Predchádzajúci kľúč**, **Prehrávanie**, **Ďalší kľúč** a **Koniec**. **Snímka** ukazuje a nastavuje aktuálnu snímku a **Trvanie (s)** dĺžku klipu. Režim je **Raz**, **Slučka** alebo **Tam a späť**.

## Časová os
Časová os má pravítko s prehrávacou hlavou a stopu pre **Poloha** (X, Y a Z s kľúčmi spolu) a pre každý uhol rotácie, RX, RY a RZ. Kliknutím alebo ťahaním v pravítku posuniete prehrávaciu hlavu a vo všetkých pohľadoch uvidíte pózu v danom čase. Ťahaním stredným tlačidlom časovú os posúvate. Kľúče v čase presúvate ťahaním; `Shift` pridáva do výberu; dvojklik do stopy vloží kľúč; `Ctrl`+koliesko približuje. **Krivky** namiesto stôp ukáže krivky hodnôt.

Nad ňou sú tlačidlá **Pridať kľúč**, **Odobrať**, **Kopírovať**, **Prilepiť** a **Duplikovať**. **Prichytávať k snímkam** drží kľúče na celých snímkach a **Prispôsobiť** ukáže celý klip. **Upraviť časovanie kľúčov** otvorí **Časová mierka**, **Posun (snímky)**, **Mierka od prehrávacej hlavy**, **Predĺžiť trvanie** a **Upraviť časovanie výberu**.

Inšpektor kľúčov vedľa časovej osi upravuje vybraný kľúč: **Snímka**, **Hodnota** (alebo tri súradnice kľúča polohy), interpoláciu **Lineárna**, **Skoková** alebo **Bézierova**, **Vstupný sklon** a **Výstupný sklon** a **Vyhladiť dotyčnice**.

## Klávesy v editore
Keď má klávesnicu časová os: `Space` spustí alebo pozastaví prehrávanie, šípky vľavo a vpravo posunú o snímku, `Ctrl+A` vyberie všetky kľúče, `Ctrl+C`, `Ctrl+X` a `Ctrl+V` kľúče kopírujú, vystrihujú a prilepujú, `Ctrl+D` ich duplikuje, `Delete` ich odoberie, `Ctrl+Z` vracia a `Esc` zruší ťahanie. Mimo časovej osi `Esc` opustí režim animácie. Vrátenie zmeny zachová priblíženie, výber aj polohu prehrávacej hlavy.

## Pózovanie v pohľadoch
Riadok pod časovou osou obsahuje **Onion skinning** s počtom a **Rozostup**, ktoré ukazujú susedné pózy ako priesvitné tiene; súradnice **Pivot**; **Automatické kľúče**; a nástroje pózy **Posunúť**, **Otočiť** a **Pivot**.

- **Pridať kľúč** zapne Automatické kľúče a pozastaví prehrávanie. So zapnutými Automatickými kľúčmi a zvoleným **Posunúť** ťahajte v 3D pohľade farebnú os úchytu vybranej entity a v aktuálnej snímke tým zaznamenáte jej polohu.
- V 2D pohľade ťahajte vybraný brush alebo člena jeho skupiny a zaznamenáte posun pozdĺž dvoch viditeľných osí s prichytávaním k mriežke. Automatické kľúče na to zapnuté byť nemusia.
- S **Otočiť** ťahajte brush v 2D pohľade okolo jeho oranžovej značky pivotu. Pohľad Zhora zaznamenáva RZ, Spredu RY a Zboku RX, s prichytávaním uhla podľa mriežky.
- S **Pivot** ťahajte žltý stred otáčania v ľubovoľnom 2D pohľade. Prichytáva sa k mriežke a zachováva hĺbku. Kľúče ani kľudová geometria sa nemenia.

Azúrová dráha v 2D aj 3D ukazuje pohyb pivotu. Kliknutím na bod dráhy vyberiete jeho čas a kľúče, ťahaním zmeníte polohu; so zapnutými Automatickými kľúčmi možno body dráhy ťahať aj v 3D pohľade. Popisky ukazujú čísla snímok a prerušované úseky označujú skoky. `Esc` ktorékoľvek z týchto ťahaní zruší a jedno vrátenie vráti celé ťahanie.

Nové animácie začínajú v pokoji s kľúčmi v čase nula. Pohyb vytvoríte zmenou pózy v inej snímke. Pre ostrú dráhu v tvare L zaznamenajte začiatok, roh a koniec a použite Lineárna s Tam a späť.

## Prepojenie tlačidiel s animáciami
Vyberte func_button, trigger_multiple alebo trigger_once. Karta Entita ukáže nad vlastnosťami **Spúšťač animácie**. Vyberte cieľ zo zoznamu, alebo kliknite na **Vybrať cieľ v pohľade** a potom na animovaný objekt v ľubovoľnom pohľade; `Esc` výber zruší. Chýbajúce ciele alebo klipy sa vypíšu pod ovládacími prvkami. Keď je zapnuté zobrazenie prepojení, entity ukazujú šípky k svojim cieľom.

**Akcia** volí, čo spustenie urobí: **Prehrať** spustí pomenovaný klip alebo vybranú animáciu; **Prepnúť** strieda `open` a `close`, ak existujú, inak znova spustí vybranú animáciu; **Otvoriť** a **Zavrieť** spustia tieto klipy; **Pozastaviť** podrží pózu, **Pokračovať** pokračuje a **Zastaviť** obnoví pôvodnú pózu. **Iba raz** prepíše predvolené správanie triedy. **Oneskorenie opakovania (s)** nastaví najkratší čas medzi spusteniami; prvé spustenie neoneskoruje.

**Vyskúšať spúšťač** vykoná akciu v náhľade bez úprav mapy a bez krokov späť. **Obnoviť náhľad** obnoví pôvodné pózy a vynuluje stav jednorazového spustenia a oneskorenia. Kolízie hráča ani herné skripty sa nesimulujú.

## Animácia v exportoch
Godot exportuje natívne zdroje AnimationPlayer a AnimationLibrary, vrátane kriviek a pohyblivej kolízie. CharacterBody3D, ktoré vstúpi do oblasti tlačidla, spustí jeho cieľ. Úpravy týchto klipov v Godote sa do mapy nevrátia a ďalší export ich nahradí.

Export pre raylib pridá `<map>_animation.h` s prehrávačom v C a `<map>_preview.c` s náhľadom s voľnou kamerou; zostavenie opisuje `<map>_raylib.md`. V náhľade `SPACE` pozastaví, `R` znova spustí predvolené klipy a `N` prepína pomenované klipy. Defold vkladá prehrávanie do skriptu entity so zdokumentovanými správami `heron_*_animation`. Build Level v Unity pridá `HeronBrushAnimation` a kinematické teleso. Export pre Bevy obsahuje `heron_animation.rs` a návod. Godot, Unity, Defold, Bevy a raylib zachovávajú prepojenia spúšťačov, príznak jednorazovosti aj oneskorenie opakovania. Vaša hra však stále musí zaregistrovať prehrávač a pripojiť vlastné tlačidlá k API každého behového prostredia.

## Príklad Motion lab
Otvorte `motion_lab` v sekcii Príklady na uvítacej obrazovke. Obsahuje plošinu s dráhou v tvare L, výťah, otáčavú plošinu a dvere na pántoch s tlačidlami. Otvorte susedný `godot/project.godot` v Godote a stlačte `F5`. WASD a myš ovládajú hráča, medzerník skáče, 1 až 4 prenesú na stanovištia a R reštartuje aktuálne stanovište. Šliapnutím na farebnú plôšku spustíte jej cieľ. Priložené README opisuje opätovný export.

# Export do enginu
Súbor → Export otvorí okno Export. Vľavo sú enginy s logami: Godot, Defold, three.js, Babylon.js, PlayCanvas, Bevy, raylib, Unity, Unreal Engine, Flax Engine, Cocos Creator, Wavefront .obj, glTF .gltf, GLB .glb a nakoniec `.map`. Stránka vpravo hovorí, čo cieľ dostane. **Zapísať** ho zapíše, **Skontrolovať** úroveň zostaví bez zápisu a vypíše problémy a **Zavrieť** okno zatvorí.

Export je oddelený od ukladania: `.map` zostáva vaším upraviteľným zdrojom. Každý ovládací prvok ukladá nastavenia svojho enginu hneď pri zmene, takže si každý engine pamätá vlastné voľby a okno sa otvorí tak, ako ste ho opustili.

## Spoločné voľby
- **Mierka**: **Jednotiek na meter**, predvolene 32.
- **Čo zapísať**: **Kolízna geometria**.
- **Optimalizácie**: **Zoskupiť statické siete do buniek s veľkosťou 1024 jednotiek**, **Zahodiť steny ukryté vo vnútri geometrie**, **Spojiť steny v rovnakej rovine** a **Spojiť kolízne tvary**.
- **Cesty projektu**: kam sa v projekte zapíše úroveň a jej zdroje.
- **Materiály nástrojov**: názvy materiálov clip, skip a origin.
- **Ďalšie**: prepínače podľa enginu, napríklad **Pomenovať uzly podľa**, **Obnoviť vrstvy a skupiny ako nadradené uzly**, **Obrátiť poradie vrcholov trojuholníkov**, prevrátenie textúrových súradníc a druhá sada UV pre zapečené svetlo s **Texel svetelnej mapy**.
- **Priebežne podľa mapy**: **Znova zapísať pri každom uložení mapy**. Je dostupné, až keď engine raz exportujete, pretože až tento export určí, kam zapisovať.

Statické bunky priradia brushe sveta podľa ich stredov a v každej bunke s veľkosťou 1024 jednotiek ich spoja podľa materiálu, takže enginy môžu vyraďovať menšie časti mapy. Kolízia zostane a herné entity zostanú oddelené. Môže to znamenať viac vykresľovacích volaní než jedna sieť sveta, preto je voľba predvolene vypnutá. Samostatné exporty OBJ, glTF a GLB bunky nepoužívajú.

## Poznámky k cieľom
- Godot: cesty `res://` k textúram a materiálom, fyzikálna vrstva spúšťačov a lightmap UV2. Zvoľte priečinok exportu vo vnútri projektu Godot.
- Defold: adresár zdrojov assetov, materiál modelu a sampler textúry, ktorý musí v zvolenom materiáli existovať.
- Unity: siete OBJ a importér pre editor; exportujte do priečinka Assets projektu.
- Unreal Engine: vygenerovaný importný skript a cesta k obsahu, obvykle /Game/<level>.
- Flax Engine: cesta k modelom relatívna k priečinku Content projektu.
- three.js, Babylon.js, PlayCanvas, Bevy, raylib a Cocos Creator: scény alebo kód pre tieto prostredia, s návodom vedľa exportu.
- glTF / GLB: prenosná geometria pre kompatibilné nástroje; GLB je binárna podoba. OBJ: geometria a materiály pre modelovacie nástroje.
- `.map`: **Zapísať ako** volí Valve 220, Štandardné, Quake 2 alebo Quake 2 (Valve). Okno povie, čo by zvolený dialekt stratil.

Po zápise si prečítajte varovania, napríklad o chýbajúcich textúrach. **Výsledky kontroly** ich vypíše priamo v okne.

# Živé prepojenie: Godot, Unity, Defold a Flax
Živé prepojenie posiela neuložené úpravy z Heron Brush do editora enginu počas práce. Je jednosmerné: úpravy urobené v engine sa nevracajú. V jednom okne mapy môže bežať jedno živé prepojenie. Pred prepnutím enginu ho zastavte a po zmene volieb exportu ho spustite znova. Ovládanie nájdete v okne Export na stránke enginu, v karte **Živé prepojenie s**.

Indikátor v titulkovom pruhu ukazuje stav prepojenia; kliknutím ho zastavíte. Zatvorenie alebo nahradenie mapy publikovanie ukončí. Po chybe importu problém opravte a prepojenie spustite znova.

## Godot
1. Raz exportujte do projektu Godot.
2. Kliknite na **Nainštalovať doplnok pre Godot**.
3. V Godote zapnite Heron Brush Live v Project Settings → Plugins.
4. Kliknite na **Spustiť živé prepojenie** a v doku Godotu pripojte mapu k otvorenej 3D scéne.

Doplnok nahrádza len svoju vygenerovanú vetvu a zachová umiestnenie pripojenia aj ostatné uzly. Scénu v Godote ukladajte sami a vlastné skripty a uzly držte mimo vygenerovanej vetvy.

## Unity
1. Raz exportujte do priečinka Assets projektu Unity.
2. Kliknite na **Nainštalovať doplnok pre Unity** a nechajte Unity doplnok skompilovať.
3. V Unity otvorte Window → Heron Brush → Live Link.
4. Spustite prepojenie a pripojte mapu k aktívnej scéne Unity.

Okno živého prepojenia v Unity nechajte otvorené. Aktualizácie sa pozastavia v Play Mode a Prefab Mode. Nahrádza sa len potomok Generated objektu mapy. Vygenerované súbory neupravujte.

## Defold
1. Raz exportujte do projektu Defold a projekt nechajte v Defolde otvorený.
2. Kliknite na **Spustiť živé prepojenie**. Nie je potrebný žiadny doplnok.
3. V Defolde otvorte heron_brush_live/preview.collection, alebo na ňu odkážte z vlastnej scény.

Prepojenie zapisuje do samostatného vygenerovaného priečinka a finálny export ani game.project nemení. **Priebežne obnovovať spustenú hru** aktualizuje aj hru spustenú z Defoldu; pridanie objektov alebo zmena kolízií môže vyžadovať reštart úrovne. Neúspešné pripojenia sa automaticky opakujú.

## Flax
Exportujte do priečinka Content projektu a kliknite na **Nainštalovať doplnok pre Flax**. Nechajte Flax doplnok skompilovať, spustite prepojenie a v otvorenej scéne Flaxu zvoľte Tools → Heron Brush: attach live map. Vygenerovaný koreň si zachová umiestnenie a ostatné objekty zostanú nedotknuté. Scénu ukladajte sami. Aktualizácie sa v Play Mode pozastavia. Tools → Heron Brush: stop live map pozastaví príjem.

# Spoločné úpravy v živej relácii
Spolupráca → Živá relácia otvorí okno na hostenie relácie alebo pripojenie k inému používateľovi Heron Brush. Takto ľudia zdieľajú úpravy mapy; je to niečo iné než živé prepojenie s enginom.

## Hostenie a pripojenie
Vyplňte **Vaše meno**. Ak chcete hostiť, zvoľte **Hostiť reláciu**, kliknite na **Spustiť reláciu** a údaje z **Kopírovať údaje na pripojenie** pošlite hosťom. Ak sa chcete pripojiť, zvoľte **Pripojiť sa k relácii**, vložte údaje a kliknite na **Pripojiť sa**. Pripojenie nahradí otvorenú mapu mapou hostiteľa, preto najprv uložte svoju prácu.

Štandardné zostavenie funguje v miestnej sieti a **Port miestnej siete** nastavuje port. Relácie cez internet (**Pripojiť cez internet**) vyžadujú zostavenie s voliteľnou sieťovou funkciou. Účastníci v 3D pohľade vidia značky kamier a mená ostatných.

Zatvorením okna relácia beží ďalej; jej stav ukazuje indikátor v titulkovom pruhu. **Zastaviť reláciu** alebo **Opustiť reláciu** ju ukončí. Keď dvaja ľudia upravujú ten istý objekt, o konečnej verzii rozhodne poradie u hostiteľa, a aj vrátenie zmeny je zdieľaná úprava. Kde sa dá, pracujte na oddelených oblastiach a ukladajte si kontrolné body.

# 3D zobrazenie a druhý monitor
## Režimy zobrazenia
Sekcia **Zobraziť** na karte Mapa prepína, čo pohľady kreslia: Brushe, Bodové entity, Modely entít, Názvy tried entít, Obrysy skupín, Patche, Materiály, Zobraziť vynechané steny, Náhľad animácií, Drôtový model, Hrany a Obrysy brushových entít. Drôtový model kreslí hrany trojuholníkov vrátane uhlopriečok a hrán za povrchmi, bez materiálov. Je to dočasné ladiace zobrazenie a nemá vplyv na exporty ani vykreslenia.

Osvetlenie, tiene, zatienenie okolím, odrazy, bloom a vyhladzovanie 3D pohľadu nájdete v Nastavenia → Renderer. Sú to nastavenia náhľadu; výsledné obrázky používajú vlastné nastavenia renderovacej kamery.

## Samostatné 3D okno
Zobraziť → 3D pohľad v samostatnom okne presunie 3D pohľad do vlastného okna, ktoré môžete dať na iný monitor. Zdieľa kameru, dokument aj nástroje a ploché pohľady zaberú miesto v hlavnom okne. Jeho ponuka obsahuje Materiály, Plné plochy a Drôtový model; podponuku Zobraziť s položkami Zobraziť vynechané steny, Náhľad animácií, Hrany, Obrysy skupín, Bodové entity, Modely entít a Obrysy brushových entít; Zaostriť na výber; Spredu, Zboku a Zhora, ktoré natočia kameru okolo výberu (alebo okolo bodu pred kamerou); Umiestniť kameru podľa 3D pohľadu; Celá obrazovka (`F11`); Vykresliť obrázok; a Vrátiť do hlavného okna. Zatvorením okna sa pohľad vráti späť.

## Procedurálna obloha
Procedurálna obloha je v 3D pohľade predvolene zapnutá a možno ju vypnúť v Nastavenia → Renderer → Svetlo. Renderovacie kamery majú vlastný prepínač **Procedurálna obloha** uložený s mapou. Farby oblohy, horizontu a zeme pochádzajú z nastavení prostredia a slnko sleduje osvetlenie scény. Obloha nemá geometriu a neexportuje sa.

# Renderovacie kamery a obrázky PNG
Súbor → Vykresliť obrázok otvorí okno vykresľovania: vľavo kamera a jej nastavenia, vpravo živý náhľad, ktorý sa prekreslí pri každej zmene. Kamery sa ukladajú do `.map` vo vrstve Render cameras, ktorá slúži len editoru a do exportov pre enginy sa nedostane.

## Umiestnenie kamery
Nastavte záber v 3D pohľade a stlačte `Ctrl+Alt+0` (alebo `Ctrl+Alt+Num0`), prípadne kliknite na **Umiestniť kameru podľa 3D pohľadu**. Ak nie je vybraná žiadna kamera, aktualizuje sa prvá uložená; ak žiadna neexistuje, vytvorí sa. Akciu možno vrátiť a zachová nastavenia výstupu aj osvetlenia. **Vybrať kameru v mape**, **Načítať kameru z mapy** a **Presunúť 3D pohľad ku kamere** fungujú opačným smerom. **Rámik kamery v 3D pohľade** ukáže hranicu výstupu a **Ovládať kameru z 3D pohľadu** nechá bežnú navigáciu hýbať uloženou kamerou.

## Nastavenia
- **Poloha kamery**: X, Y, Z, natočenie a sklon.
- **Výstup a objektív**: **Predvoľby** (Full HD, Štvorec, Na výšku), šírka, výška, vzorky, **Zvislý zorný uhol** a **Vzdialená orezová rovina**.
- **Osvetlenie**: Denné svetlo, Západ slnka alebo Zamračené, prípadne expozícia, slnko, farby oblohy, horizontu a zeme, **Procedurálna obloha**, **Tiene**, **Zatienenie okolím** a **Bloom**.
- **Hĺbka ostrosti**: **Zapnúť hĺbku ostrosti**, vzdialenosť zaostrenia, šírka ostrého pásma a polomer rozostrenia. **Zaostriť na výber** zaostrí na stred vybraného objektu pred kamerou.

**Uložiť kameru a nastavenia** zapíše formulár do mapy; aby zostal na disku, uložte mapu.

## Vykresľovanie
**Vykresliť PNG** sa opýta, kam uložiť, a spustí postupné vykresľovanie, ktoré sa v náhľade dopĺňa vzorka po vzorke. Viac vzoriek vyzerá lepšie, ale trvá dlhšie. **Zrušiť vykresľovanie** skončí bez zápisu súboru. Vykresľovanie používa rastrový postup editora, nie sledovanie lúčov. PNG má nepriehľadné pozadie a animácie sa nevykresľujú. Veľmi veľké obrázky môžu editor na chvíľu pozastaviť.

# Nastavenia
Súbor → Nastavenia (`Ctrl+,`) otvorí nastavenia. Zmeny platia okamžite a ukladajú sa. Bočný panel obsahuje Prehľad, Hry, Zobraziť, Farby, Myš, Klávesnica a Renderer. Pole na hľadanie navrchu nájde nastavenie podľa názvu alebo popisu na ktorejkoľvek stránke. Každé zmenené nastavenie má vlastné tlačidlo na obnovenie predvolenej hodnoty a **Obnoviť predvolené** obnoví celú aktuálnu stránku.

## Stránky
- **Prehľad**: bežné nastavenia pohromade: Vzhľad (Krytie panelov (%), Farebná schéma, Rozloženie, Jazyk), Úpravy, Zobraziť a Príručka.
- **Hry**: hra, kde sú jej súbory a vaše meno pre spoluprácu. Hra určuje, aký formát mapy zapisovať, ktoré definície entít načítať a kde sú jej materiály. Konfigurácie hier sa čítajú z priečinka `games` v priečinku konfigurácie (`~/.config/heron-brush/games` na Linuxe a macOS, `%LOCALAPPDATA%\heron-brush\games` na Windows) a z priečinka `games` vedľa aplikácie, kam balíky ukladajú pribalenú konfiguráciu. Ak v priečinku hry chýba priečinok, ktorý uvádza konfigurácia hry, napríklad `baseq3` z Quake 3 v inštalácii OpenArena, Heron použije náhradu, ktorú konfigurácia pozná (`baseoa`), inak jediný priečinok s balíkmi `.pak` alebo `.pk3`, a oznámi to pod **Mody**. Priečinky uvedené v **Mody** sa aj tak pripoja nad neho.
- **Zobraziť**: Rozloženie, Farebná schéma, Jazyk, osi sveta, krytie mriežky, veľkosť bunky kontroly netesností a zámky mapovania.
- **Farby**: farby pohľadov, geometrie a úchytov.
- **Myš**: rýchlosti a prevrátenie rozhliadania, posunu a pohybu, rýchlosť letu, zorný uhol, Prepojiť 2D pohľady a veľkosti úchytov.
- **Klávesnica**: klávesové skratky, Heron Brush alebo Blender.
- **Renderer**: realistické tieňovanie, obrys výberu, svetlo, tiene, zatienenie okolím, odrazy, bloom a povrchy.

## Jazyk
**Jazyk** ponúka Podľa systému a všetky preložené jazyky. Rozhranie sa zmení okamžite. Príručka sa otvorí v jazyku rozhrania, ak pre neho existuje preklad, inak v angličtine.

# Pluginy a MCP
## Pluginy
Pluginy rozširujú editor o skripty v jazyku Rhai. Súbor → Pluginy otvorí okno Pluginy. **Nainštalovať plugin** nainštaluje súbor `.rhai` a **Nainštalovať priečinok pluginu** priečinok, ktorý obsahuje `main.rhai`; voliteľné metadáta `plugin.toml` opisujú plugin a jeho nastavenia. Pluginy sú rozdelené na **Aktívne pluginy** a **Neaktívne pluginy** a **Zapnúť plugin** plugin zapne alebo vypne; voľba prežije reštart. **Znova načítať pluginy** znova prečíta skripty z disku a vynuluje hodnoty v ich paneloch.

Aktívne pluginy sa zobrazujú aj v sekcii **Pluginy** na karte Mapa s nastaveniami, tlačidlami akcií (**Spustiť**) a výstupom. Akcia pracuje s výberom, alebo so všetkým, keď nie je nič vybrané, a je jedným krokom späť. Pribalený plugin Selection inspector ukazuje počty a akcie na posun nahor a nadol.

## MCP
Spolupráca → Sprístupniť agentovi (MCP) umožní AI agentovi alebo inému nástroju ovládať otvorený editor cez Model Context Protocol. Okno ukazuje miestnu adresu s tlačidlom **Kopírovať** a **Zastaviť server** ho ukončí; kým beží, titulkový pruh ukazuje **MCP aktívne**. Voľba príkazového riadka `--mcp` spustí rovnaké rozhranie bez okna. Klienti môžu zistiť dostupné nástroje pre geometriu, assety, vlastnosti, animácie, export a relácie. MCP a pluginy editora sú dve rôzne rozhrania.

# Príkazový riadok
Program sa volá `heron-brush`. Bez volieb otvorí uvítaciu obrazovku.

## Otváranie súborov
`heron-brush map.map [entities.fgd] [materials/]` otvorí mapu, voliteľne s definíciami entít a priečinkom materiálov. Prázdny argument jeden vynechá, napríklad `heron-brush map.map "" materials/`. Ak editor už beží, mapa sa otvorí v ňom; `--new-instance` namiesto toho otvorí samostatné okno.

## Príkazy
- `--help` alebo `-h`: vypíše použitie. `--version` alebo `-V`: vypíše verziu.
- `--mcp`: obsluhuje MCP cez štandardný vstup a výstup, bez okna.
- `--merge-map <base> <ours> <theirs> [<output>]`: trojcestné zlúčenie súborov `.map` pre git. Výstup je predvolene ours. Konflikty sa vypíšu a vrátia nenulový návratový kód.
- `--obj-to-map <in.obj> <out.map> [size] [thickness] [snap] [material]`: prevedie sieť na brushe. Size je dĺžka najdlhšej strany v jednotkách mapy (predvolene 256), snap je mriežka pre rohy (0 pre žiadnu) a slová `solid` alebo `layered` namiesto tenkých klinov vytvoria kvádre v mriežke alebo konvexné vrstvy.
- `--render-preview <in.map> <out.png> [size]`: zapíše náhľad mapy ako PNG, predvolene 512 pixelov.
- `--run-script <script.rhai> <in.map> [out.map] [key=value ...]`: spustí skript pluginu nad mapou so všetkým vybraným a zapíše výsledok (predvolene späť do vstupu). `key=value` odpovedá na nastavenia pluginu.

# Riešenie problémov
## Keď niečo nevyzerá dobre
- Chýba materiál: skontrolujte priečinok alebo balík materiálov a cesty projektu v exporte.
- Nesprávna veľkosť modelu: porovnajte so 64-jednotkovou značkou hráča a mierkou exportu.
- Nesprávny smer prechodu: skontrolujte uhol prechodu a orientáciu steny.
- Prázdna scéna v engine: prečítajte si varovania exportu a postupujte podľa pokynov na import pre daný cieľ.
- Živé prepojenie čaká: overte, že je doplnok nainštalovaný, scéna pripojená a aktualizácie nie sú pozastavené.
- Len čiary bez stien: vypnite Drôtový model v sekcii Zobraziť na karte Mapa, alebo v ponuke 3D okna zvoľte Materiály či Plné plochy.
- V pohľadoch chýbajú objekty: `Ctrl+Shift+I` zobrazí skryté objekty; skontrolujte vrstvy a filtre zobrazenia.
- Kláves nič nerobí: najprv kliknite do pohľadu, aby mal klávesnicu, a v Nastavenia → Klávesnica skontrolujte klávesové skratky.
- Úprava zlyhala: vráťte ju, zjednodušte geometriu a použite **Skontrolovať mapu** a **Skontrolovať netesnosti** v sekcii **Problémy** na karte Mapa.
- Mapa sa neotvorí zo správcu súborov: mohla byť odovzdaná už bežiacemu editoru; hľadajte jeho okno.

## Nastavenia a zálohy
Nastavenia, nedávne mapy, hviezdičky, nastavenia exportu a pluginy sú v priečinku konfigurácie: `~/.config/heron-brush` na Linuxe a macOS a `%LOCALAPPDATA%\heron-brush` na Windows. Zálohy automatického ukladania sú v priečinku `autosave` vedľa každej mapy.

## Hlásenie chýb
Uveďte verziu editora (Pomocník → O programe), operačný systém, malú mapu, na ktorej je problém vidieť, a presný postup. Výstupný panel (`Ctrl+4`) často povie, čo sa pokazilo. Pred experimentovaním si uložte kópiu.

## Ako používať túto príručku
Kapitolu zvoľte v obsahu vľavo. `Ctrl+F` zobrazí obsah a presunie vás do poľa na hľadanie; `Esc` hľadanie vymaže, alebo príručku zatvorí, keď je pole prázdne. **Obsah** skryje alebo zobrazí zoznam kapitol a **Veľkosť textu** mení veľkosť písma; obe voľby sa pamätajú. Zoznam jazykov navrchu prepína medzi dostupnými prekladmi.
