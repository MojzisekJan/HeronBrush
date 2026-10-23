# Aan de slag
Heron Brush is een leveleditor die met brushes werkt. Je bouwt levels uit convexe brushes, beschildert en textureert hun vlakken, plaatst entiteiten, animeert deuren en platforms en exporteert het resultaat naar een game-engine. Daarnaast zijn er ingebouwde assets, parametrische vormen, livekoppelingen met engine-editors, gedeelde bewerkingssessies en het renderen van stilstaande beelden. Deze handleiding wordt met de applicatie meegeleverd en werkt offline.

Op macOS werkt `Cmd` overal waar deze handleiding `Ctrl` noemt, en Option is Alt.

## Het welkomstscherm
Als Heron Brush zonder map start, opent het welkomstscherm. Links staan **… voortzetten** (de map die je het laatst open had), **Nieuwe map** en **Bladeren** om een bestaande `.map` te openen, plus **Spel**, **Handleiding** en **Voorkeuren**. Rechts staat een lijst met recente maps en miniaturen, gevolgd door **Voorbeelden**. De ster (**Bovenaan houden**) zet een map vast bovenaan de lijst; de prullenbak (**Naar de prullenbak verplaatsen**) verplaatst het bestand naar de prullenbak van het systeem, zodat je het kunt terugzetten. Met de pijltoetsen beweeg je over het scherm en Enter opent het gemarkeerde item. Bestand → Welkom brengt het scherm op elk moment terug.

## Je eerste map
1. Kies **Nieuwe map** op het welkomstscherm, of Bestand → Nieuwe map (`Ctrl+N`).
2. Kies het Brush-gereedschap (`B`) in de gereedschapsbalk en sleep in een 2D-weergave om de gekozen vorm te tekenen. Controleer de diepte in een andere 2D-weergave.
3. Selecteer de brush en verplaats hem door te slepen. Gebruik het tabblad **Vlak** om hem een materiaal of een kleur te geven.
4. Sla de bronmap op met `Ctrl+S`. Wil je hem in een spel gebruiken, open dan Bestand → Exporteren en schrijf een scène voor je engine.

## Bestanden en opslaan
Het `.map`-bestand is je bewerkbare bron. Sla regelmatig op: exporteren, livekoppelingen en renderen slaan de map nooit voor je op. Bestand → Opslaan als (`Ctrl+Shift+S`) voegt de extensie `.map` toe als je die weglaat. Bestand → Terug naar opgeslagen map gooit niet-opgeslagen wijzigingen weg en laadt het bestand opnieuw. Sluit je een venster of open je een andere map terwijl er niet-opgeslagen wijzigingen zijn, dan vraagt Heron Brush of je wilt **Opslaan**, **Verwerpen** of **Annuleren**.

Zolang een opgeslagen map niet-opgeslagen wijzigingen heeft, worden er elke tien minuten genummerde back-ups geschreven naar een map `autosave` naast de map. De 50 nieuwste back-ups worden bewaard.

Open je een map terwijl Heron Brush al draait, dan wordt die doorgegeven aan het draaiende venster, dat naar voren komt. Gebruik `--new-instance` op de opdrachtregel als je een apart venster wilt (zie Opdrachtregel).

# De interface
Het hoofdvenster heeft een titelbalk met de menu's, daaronder een gereedschapsbalk, de mapweergaven, rechts de inspector en onderaan een statusbalk.

## Titelbalk en menu's
De titelbalk toont het logo, de menu's, de mapnaam in het midden en de vensterknoppen. Op macOS staan de gebruikelijke rode, gele en groene knoppen links. Sleep de titelbalk om het venster te verplaatsen en dubbelklik erop om het te maximaliseren. Terwijl er iets op de achtergrond draait, verschijnt er een indicator aan het eind van de balk: een livekoppeling met een engine, een livesessie of **MCP actief**. Klik op een indicator om de livekoppeling te stoppen of het bijbehorende venster te openen.

- **Bestand**: Welkom, Nieuwe map (`Ctrl+N`), Map openen (`Ctrl+O`), Opslaan (`Ctrl+S`), Opslaan als (`Ctrl+Shift+S`), Terug naar opgeslagen map, Exporteren, Afbeelding renderen, Plug-ins en Voorkeuren (`Ctrl+,`).
- **Bewerken**: Ongedaan maken (`Ctrl+Z`), Opnieuw (`Ctrl+Shift+Z`), Knippen, Kopiëren, Plakken, Dupliceren (`Ctrl+D`), Verwijderen, Alles selecteren (`Ctrl+A`), Laatste opdrachten herhalen (`Ctrl+R`) en Herhaalbare opdrachten vergeten.
- **Beeld**: 3D-weergave in apart venster, Ga naar, Aan raster kleven (`Alt+0`), Raster tonen (`0`), Voorvertoning van animaties en Weggelaten vlakken tonen.
- **Geometrie**: 3D-tekst invoegen, Hoek verbinden, Vlakken naar patches omzetten, Selectie uit haar entiteit halen en Gekoppelde groepen synchroniseren.
- **Animatie**: Keyframe-animatie, Voorvertoning van animaties en Camera plaatsen vanuit 3D-weergave (`Ctrl+Alt+0`).
- **Map**: Puntbestand laden, opnieuw laden en sluiten; Portaalbestand laden, opnieuw laden en sluiten. Deze tonen leksporen en portalen die door mapcompilers zijn geschreven.
- **Samenwerking**: Livesessie en Aanbieden voor een agent (MCP).
- **Help**: Handleiding en Over.

## De gereedschapsbalk
De gereedschapsbalk bevat alle gereedschappen. De eerste groep gebruik je voortdurend: Selecteren, Brush, Knippen, Vertex, Rand, Vlak, Roteren en Schalen. De tweede groep bevat Schuintrekken, Samenstellen, Vegen, Controlepunt, Afronden en Verven. Houd de muis boven een knop om de naam en sneltoets te zien. Klik je nogmaals op het actieve gereedschap, dan keer je terug naar Selecteren.

Na de gereedschappen volgen **Horizontaal spiegelen** en **Verticaal spiegelen**, en dan twee schakelaars: **Vergrendelen** behoudt de materiaalprojectie als een brush wordt verplaatst, en **UV** behoudt haar als een brush draait (`U` schakelt dit om). Daarna komen de instellingen van het actieve gereedschap: het Brush-gereedschap toont de vorm en de opties ervan, en het Afrondgereedschap toont straal, vlakken en hoeken met **Toepassen**. Helemaal rechts staan de naam van het gereedschap, de rastergrootte (**Raster 16** enzovoort; klik om een andere te kiezen) en het menu **Rasterinstellingen** met Aan raster kleven, Raster tonen, Voorvertoning van animaties, Weggelaten vlakken tonen en Keyframe-animatie.

## Mapweergaven
Er zijn vier weergaven: de 3D-weergave en drie platte weergaven, Bovenkant (XY), Voorkant (XZ) en Zijkant (YZ). Voorkeuren → Beeld → Indeling kiest Eén venster, Twee vensters, Drie vensters of Vier vensters. Sleep de scheidingslijnen om de vensters te vergroten of verkleinen. Klik in een weergave om haar het toetsenbord te geven. `Space` schakelt een venster dat één weergave tegelijk toont door naar de volgende weergave, en `Ctrl+Space` maximaliseert de weergave met het toetsenbord en herstelt daarna de indeling weer.

Klik met de rechtermuisknop zonder te slepen in een weergave om een contextmenu te openen voor de selectie en wat er onder de aanwijzer ligt. Het biedt groeperen, lagen, gekoppelde groepen, entiteiten maken, brushes in of uit een entiteit verplaatsen, een materiaalnaam kopiëren en het materiaal tonen in de materiaalbrowser.

## De inspector
De inspector rechts heeft vier tabbladen. Sleep de linkerrand om de breedte te veranderen.

- **Map** (`Ctrl+1`): Lagen, Groepen, Plug-ins, Problemen, weergavefilters en Tags.
- **Entiteit** (`Ctrl+2`): de entiteitsbrowser, de entiteitsdefinities, de eigenschappen van de geselecteerde entiteit en, voor knoppen en triggers, Animatietrigger.
- **Vlak** (`Ctrl+3`): UV, Materialen, Kleur, Vegen, Afronden, Patch en Inzetten en afschuinen. Elke sectie kan worden ingeklapt.
- **Assets**: de bibliotheek met herbruikbare brushmodellen.

`Ctrl+5` verbergt of toont de inspector en `Ctrl+Alt+T` verbergt of toont de gereedschapsbalk, zodat de weergaven meer ruimte krijgen.

## De statusbalk en het uitvoerpaneel
De statusbalk onderaan toont het laatste bericht van de editor. Fouten worden rood weergegeven. Klik op de balk of druk op `Ctrl+4` om het uitvoerpaneel te openen met de volledige berichtgeschiedenis. De meeste opdrachten melden hier wat ze hebben gedaan, bijvoorbeeld hoeveel brushes er zijn geknipt.

## Dialoogvensters en uiterlijk
Dialoogvensters zoals Voorkeuren, Exporteren en Afbeelding renderen openen als eigen vensters. Voorkeuren → Beeld → Kleurenschema kiest Bureaublad volgen, Licht of Donker. **Dekking van panelen (%)** op de pagina Overzicht maakt de panelen boven de weergaven doorzichtig. Het venster tekent zijn eigen knoppen in de stijl van je systeem: native op macOS, in Windows 11-stijl op Windows en in GNOME-stijl op Linux.

# Navigatie en toetsenbord
## Bewegen in de 3D-weergave
Houd de rechtermuisknop ingedrukt en sleep om rond te kijken. Sleep met de middelste knop om te pannen. Het muiswiel beweegt naar voren en naar achteren. `W`, `A`, `S` en `D` vliegen naar voren, links, achteruit en rechts, `Q` stijgt en `X` daalt; houd `Shift` ingedrukt voor grotere stappen. Staat **Alt + slepen met middelste knop verplaatst de camera** aan in Voorkeuren → Muis, dan beweeg je met Alt en de middelste knop langs de kijkrichting. Snelheden, omkeringen en het gezichtsveld vind je in Voorkeuren → Muis.

## Bewegen in 2D-weergaven
Sleep met de middelste knop om te pannen en gebruik het muiswiel om rond de aanwijzer te zoomen. Pannen en zoomen in één platte weergave houdt de andere uitgelijnd; **2D-weergaven koppelen** in Voorkeuren → Muis regelt dit. `Ctrl+Alt+Z` zet de zoom terug en `Ctrl+Shift+U` zet de platte weergaven terug naar de oorsprong.

`Ctrl+U` brengt de selectie in elke weergave in beeld. Beeld → Ga naar (`Ctrl+Alt+M`) verplaatst de camera naar ingetypte coördinaten, selecteert wat op een regel van het mapbestand staat, of verplaatst de selectie over een ingetypte afstand.

## Raster en kleven
`+` en `-` veranderen de rastergrootte, en de cijfertoetsen `1` tot `9` kiezen direct een grootte, van 1 tot 256 eenheden. `0` of `#` toont of verbergt het raster, en `Alt+0` zet het kleven aan of uit. `S` laat de vertices van de geselecteerde brushes aan het huidige raster kleven, `Ctrl+Shift+V` aan hele eenheden en `Ctrl+Alt+Shift+V` aan het raster.

## Veelgebruikte sneltoetsen
- `F3` of `Ctrl+Shift+P`: zoek elke menuopdracht op naam en voer hem uit met `Enter`. `F10` opent de menu's en de pijltoetsen bewegen erdoorheen.
- `Ctrl+S`: map opslaan. `Ctrl+Shift+S`: opslaan als.
- `Ctrl+Z`: ongedaan maken. `Ctrl+Shift+Z` of `Ctrl+Y`: opnieuw.
- `Ctrl+C`, `Ctrl+X`, `Ctrl+V`: kopiëren, knippen en plakken. Plakken zet de objecten bij de aanwijzer; `Ctrl+Alt+V` behoudt hun oorspronkelijke coördinaten. Zijn er vlakken geselecteerd, dan kopieert kopiëren hun materiaal en UV-instellingen.
- `Ctrl+D`: dupliceren. `Ctrl+Shift+D`: gekoppeld duplicaat van een groep.
- `Delete` of `Backspace`: de selectie verwijderen.
- `Esc`: het huidige slepen annuleren, een stap terug in het actieve gereedschap, terug naar Selecteren en ten slotte de selectie opheffen. `Shift+Esc` gaat direct terug naar Selecteren.
- `Ctrl+R`: de laatste herhaalbare opdrachten herhalen. `Ctrl+Shift+R`: ze vergeten.
- `Ctrl+Alt+0` of `Ctrl+Alt+Num0`: de rendercamera op de 3D-weergave plaatsen.
- `F5`: de materialen opnieuw laden. `F6`: de entiteitsdefinities opnieuw laden.

## Sneltoetsen voor selecteren
- `Ctrl+A`: alles selecteren, of alle handgrepen terwijl een vertex-, rand- of vlakgereedschap actief is. `Ctrl+Shift+A`: niets selecteren. `Ctrl+Alt+A`: de selectie omkeren.
- `Ctrl+B`: de andere objecten in dezelfde entiteit of groep selecteren.
- `Ctrl+T`: alles selecteren wat de geselecteerde brushes raken. `Ctrl+E`: alles erbinnen selecteren. `Ctrl+Shift+E`: alles in hun kolom langs de kijkas selecteren. Hierbij worden de selecterende brushes verbruikt.
- `Ctrl+I`: de selectie isoleren. `Ctrl+Alt+I`: haar verbergen. `Ctrl+Shift+I`: alles weer tonen.
- `Ctrl+G`: groeperen. `Ctrl+Shift+G`: groepering opheffen.

## Sneltoetsen voor transformeren
- De pijltoetsen en `Page Up` / `Page Down` verplaatsen de selectie één rasterstap in de richting waarin ze in de actieve weergave wijzen. Met `Ctrl` dupliceren ze tijdens het verplaatsen, met `Alt` roteren ze.
- `Alt+X`, `Alt+Y`, `Alt+Z`: een kwartslag om die as draaien; voeg `Shift` toe voor de andere richting.
- `Ctrl+F`: spiegelen over de weergave; `Ctrl+Alt+F`: verticaal spiegelen. `Ctrl+Shift+X` en `Ctrl+Shift+Y` spiegelen langs X of Y.
- `Insert`: een puntentiteit in het midden van de selectie plaatsen.
- `Tab`: met een brush geselecteerd doorlopen van Selecteren → Vertex → Rand → Vlak → Selecteren.

## De Blender-sneltoetsen
Voorkeuren → Toetsenbord → Sneltoetsen biedt Heron Brush (de standaard, die in deze hele handleiding wordt gebruikt) en Blender. Met Blender: `G` selecteert en verplaatst, `R` roteert, `S` schaalt, `E` veegt, `1`, `2` en `3` kiezen het vertex-, rand- en vlakgereedschap, `K` knipt, `B` selecteert, `A` selecteert alles en `Alt+A` heft de selectie op, `X` verwijdert, `Shift+D` dupliceert, `H` verbergt en `Alt+H` toont alles, en `Ctrl+Shift+Z` voert opnieuw uit. Al het andere is in beide gelijk.

# Brushes en modelleergereedschappen
Brushes zijn convexe lichamen die door vlakken worden begrensd. Bouw complexe objecten uit meerdere eenvoudige brushes in plaats van te proberen één brush concaaf te maken. Houd het aantal brushes laag waar een eenvoudige vorm volstaat.

## Brushes tekenen
Kies het Brush-gereedschap (`B`) en sleep in een weergave. De gereedschapsbalk toont **Te tekenen vorm**: Balk, Trap, Poortboog, Cilinder, Kegel, Bol (UV), Bol (icosaëder) en 3D-tekst invoegen. `Alt+1` tot `Alt+7` kiezen direct een vorm. Afhankelijk van de vorm toont de balk **Uithollen** en **Dikte:**, **Trede-hoogte:** en **Richting:** voor trappen, **As:**, **Zijden:** en de cirkelmodus voor ronde vormen, **Ringen:** voor bollen en **Nauwkeurigheid:** voor de icosaëder. Stel de opties in voordat je sleept. Controleer het resultaat in meer dan één weergave.

## Selecteren
Klik op een brush om hem te selecteren, en `Ctrl`-klik om hem aan de selectie toe te voegen. Slepen vanuit een lege plek tekent een selectiekader. In de 3D-weergave selecteert `Shift`-klik één vlak en voegt `Ctrl+Shift`-klik meer vlakken toe. Slepen aan een geselecteerde brush verplaatst hem; slepen aan een niet-geselecteerde brush selecteert en verplaatst alleen die brush.

In de 3D-weergave extrudeer je een vlak van een geselecteerde brush met `Shift`-slepen. `Alt`-klik op een ander vlak kopieert het materiaal en de uitlijning van het geselecteerde vlak daarnaartoe; `Alt+Ctrl` kopieert alleen het materiaal en `Alt+Shift` kopieert de rotatie.

## Vormgereedschappen
- **Selecteren** (`Esc`): selecteren en verplaatsen.
- **Vertex** (`V`), **Rand** (`E`), **Vlak** (`F`): hoeken, randen of vlakken verslepen. `Shift`-slepen vanuit het midden van een rand of vlak voegt daar een hoekpunt toe. `Delete` verwijdert de gekozen hoekpunten.
- **Roteren** (`R`), **Schalen** (`T`), **Schuintrekken** (`G`): de selectie met handgrepen transformeren.
- **Knippen** (`C`): klik twee of drie punten aan om een snijvlak te bepalen. `Enter` snijdt, `Ctrl+Enter` kiest welke kant blijft, en `Backspace` of `Esc` verwijdert het laatste punt.
- **Samenstellen** (`N`): plaats punten rond een volume en druk op `Enter` om er een convexe brush van te maken. `Backspace` verwijdert het laatste punt.
- **Vegen** (`Y`): zie de sectie over vegen hieronder.
- **Controlepunt** (`P`): het controlerooster van een patch vervormen.
- **Afronden** (`O`): klik op een rand om hem af te ronden, of kies randen of vertices met hun gereedschappen en druk op `Enter`. Meer vlakken geven een gladder resultaat en meer geometrie.
- **Verven** (`K`): zie Materialen en UV's.

## Brushes combineren
- `Ctrl+J`: de geselecteerde brushes samenvoegen tot hun convexe omhulsel.
- `Ctrl+K`: de geselecteerde brushes aftrekken van de brushes die ze raken.
- `Ctrl+Shift+K`: de selectie uithollen, met wanden van één rasterstap dik.
- `Ctrl+L`: de doorsnede van de selectie nemen.

## Vegen, afronden, patches en afschuinen
**Vegen** bouwt geometrie langs een pad vanuit geselecteerde vlakken. Selecteer de vlakken (in het veeggereedschap met `Alt`-klik), stel dan in de sectie Vegen van het tabblad Vlak **Traject**, **Draaiing** (as en hoek), **Schalen**, **Treden** en **Pad** in (uitlijning Geheel getal of Vrij; Boog, Recht of S-bocht), en druk op **Vegen**. De regel onder de bedieningselementen zegt hoeveel brushes er worden gemaakt.

De sectie **Afronden** rondt de gekozen randen af met **Straal**, **Vlakken** en **Hoeken** (Verstek of Afgerond); met één vlak ontstaat een schuine kant.

**Patch** toont **Kolommen** en **Rijen** van de geselecteerde patch, met knoppen om een paar toe te voegen of te verwijderen. Geometrie → Vlakken naar patches omzetten (`Ctrl+P`) maakt patches van geselecteerde vlakken.

**Inzetten en afschuinen** werkt op geselecteerde vlakken: **Inzetten** trekt het vlak vanaf zijn randen naar binnen en **Diepte** duwt het naar buiten (of naar binnen bij een negatieve waarde). **Vlakken afschuinen** past beide toe; de derde knop voegt een paneel toe of snijdt een uitsparing, afhankelijk van het teken van Diepte. **Hoek verbinden** opent een dialoogvenster dat twee brushes met voorvertoning verbindt in een verstek- of afgeronde hoek.

## 3D-tekst
Geometrie → 3D-tekst invoegen (ook het laatste item in de vormenlijst van het Brush-gereedschap) bouwt brushes uit tekst. Het gebruikt Heron Block, een ingebouwd polygoonlettertype met afgeschuinde hoeken, proportionele breedtes en Tsjechische accenten; kleine letters gebruiken de vormen van hoofdletters. Typ de **Tekst**, stel **Hoogte** (de hoofdletterhoogte; accenten steken erboven uit) en **Diepte** in, bekijk de live voorvertoning en druk op **Invoegen**.

Kan een bewerking geen geldige geometrie opleveren, pas dan de afmetingen aan, of maak de bewerking ongedaan en vereenvoudig de vorm.

## Arrays
Geometrie → Array kopieert de selectie en toont de kopieën in de kaart terwijl je de waarden wijzigt. De aantallen tellen het origineel mee, en een array maakt hoogstens 1024 kopieën.

- **Rij** verschuift elke kopie ten opzichte van de vorige met **Stap X**, **Stap Y** en **Stap Z** en draait haar **Draaiing per kopie (graden)** om de gekozen as.
- **Raster** vult **Aantal X**, **Aantal Y** en **Aantal Z** items, **Stap X**, **Stap Y** en **Stap Z** uit elkaar.
- **Ring** verdeelt de items over **Hoek (graden)** rond een middelpunt op **Straal (eenheden)** van de selectie. Met **Naar het midden draaien** draait elke kopie mee met de ring.
- **Verspreiding** zet de kopieën op willekeurige plekken binnen de straal, dwars op de gekozen as. Dezelfde **Seed** geeft dezelfde indeling, en **Willekeurige draaiing** draait elke kopie.

Als de selectie één hele groep is, maakt **Gekoppelde kopieën** van elke kopie een gekoppeld duplicaat, zodat een wijziging aan één ze allemaal bereikt. **Toepassen** houdt de kopieën en selecteert ze samen met het origineel; **Annuleren** verwijdert ze. De hele array wordt in één stap ongedaan gemaakt.

## Terrein boetseren
Terrein bestaat uit veel kleine brushes waarvan het boetseerpenseel het oppervlak naar buiten duwt en naar binnen trekt. Selecteer een of meer brushes en kies Geometrie → In kolommen verdelen. De brushes worden langs de rasterlijnen in kolommen van één rasterstap breed gesneden, die in een groep met de naam Terrain komen. De sneden volgen het raster van de wereld, dus brushes naast elkaar worden op dezelfde plekken gesneden en delen hun hoeken. Elke kolom staat langs de dunne kant van de brush: rechtop in een vloer of een plafond, liggend in een wand. Een grot werkt net zo: hol een blok uit (Hollow selection) met een wanddikte van één rasterstap, selecteer de wanden en verdeel ze.

Het venster **Boetseren**, dat de knop aan het eind van de boetseerbalk opent, kiest de vorm van de kolommen en kan de selectie ook splitsen. **Driehoek** maakt er twee per rastercel, en de bovenkant van een driehoek blijft vlak hoe je hem ook boetseert. **Vierkant** maakt er één per cel, dus half zoveel brushes; de bovenkant buigt langs een diagonaal. **Zeshoek** bedekt de brush met zeshoeken van twee rasterstappen breed, elk verdeeld in zes driehoeken rond het midden, zodat elke bovenkant vlak blijft.

Kies het gereedschap **Boetseren** (`J`) en sleep in de 3D-weergave over het oppervlak. De oranje ring toont het penseel. Zonder selectie verandert het penseel elke brush eronder; met een selectie alleen de geselecteerde. De balk bovenaan het venster kiest wat een streek doet:

- **Verhogen** en **Verlagen** bewegen het oppervlak naar buiten of naar binnen, het meest in het midden van de ring en niet aan de rand. Houd `Shift` ingedrukt om ze te wisselen.
- **Gladmaken** strijkt bulten glad.
- **Egaliseren** brengt het oppervlak op het niveau waar de streek begon.

In hetzelfde venster kiest **Richting** welke kant op:

- **Omhoog** beweegt het oppervlak recht omhoog en omlaag, binnen een cirkel op de grond. Dit is voor terrein.
- **Camera** beweegt het naar de weergave toe en ervan af, binnen een bol rond de aanwijzer. Kijk naar een vloer, een wand of een plafond en het penseel vormt wat je ziet.
- **Normaal** beweegt het naar buiten uit het vlak onder de aanwijzer waar de streek begint.

De twee getallen zijn **Straal (eenheden)** en **Sterkte (%)**. Alleen de hoeken aan de kant die de streek beweegt, bewegen mee; de achterkant blijft waar hij is en een kolom blijft minstens één eenheid dik. Waar brushes een hoek delen, bewegen ze die samen, zodat het oppervlak gesloten blijft. Een streek wordt in één stap ongedaan gemaakt. In de 2D-weergaven selecteert het gereedschap zoals het Selectiegereedschap.

Als het boetseren klaar is, selecteer je het terrein en kies je Geometrie → **Kolommen samenvoegen**, of de knop in het venster **Boetseren**. Naburige kolommen die samen een convexe brush vormen en er hetzelfde uitzien, worden één brush, zodat vlakke stukken een paar grote brushes worden. Het oppervlak verschuift niet, en het samenvoegen maak je in één stap ongedaan.

Voor terrein zoals in voxelgames splits je in **Vierkant**-kolommen en kies je **Blokken** bij **Streek** in het venster **Boetseren**. Een streek verplaatst dan de hele bovenkant van elke kolom in hele rasterstappen en houdt de zijkanten recht, zodat de grond in treden stijgt. Alle modi werken als voorheen; kleine tikken tellen op tot een hele stap. Kolommen van andere vormen blijven zoals ze zijn.

**Vrij** heeft geen kolommen nodig: het werkt op elke brush. Verhogen voegt grond toe in een bol rond de aanwijzer en Verlagen haalt die weg, zodat de grond kan overhangen en uitgehold kan worden tot grotten. Gladmaken egaliseert en Egaliseren brengt alles naar het niveau waar de streek begon. Elke tik bouwt de brushes die hij raakt opnieuw op het raster, afgerond, en laat alles buiten zijn bol zoals het was, scherpe randen inbegrepen. Het resultaat is gewone brushes, waarop elk ander gereedschap werkt, en **Kolommen samenvoegen** maakt het er daarna minder. Een fijner raster geeft een gladder oppervlak en meer brushes.

# Materialen en UV's
## Materialen laden en toepassen
Open de sectie **Materialen** in het tabblad Vlak. **Map laden** voegt een map met afbeeldingen toe en **.wad laden** voegt een texturepakket toe; beide verschijnen onder **Collecties**, waar het kruisje er een verwijdert. Typ in **Materialen filteren** om de lijst te beperken. Klik op een materiaal om het actueel te maken; **Op selectie toepassen** zet het dan op elk geselecteerd vlak. **Selecteren** biedt Brushes ermee selecteren en Vlakken ermee selecteren. **Vervangen** wisselt een materiaal in de hele map of alleen in de selectie om voor een ander.

Het gereedschap **Verven** (`K`) zet het actuele materiaal op het vlak onder de aanwijzer: een klik verft één vlak, een dubbelklik verft de hele brush, en `Shift`-klik neemt het materiaal van het vlak over als actueel materiaal, zoals een pipet.

## De UV-editor
De sectie **UV** in het tabblad Vlak toont het materiaal en de omtrek van het geselecteerde vlak. Muisbediening op het canvas:
- Het muiswiel zoomt rond de aanwijzer. Sleep met de rechter- of middelste knop om te pannen. Dubbelklik met de rechterknop om het vlak weer in beeld te brengen; selecteer je een ander vlak, dan gebeurt dat automatisch.
- Slepen met links verplaatst het materiaal terwijl de omtrek van het vlak blijft staan. De ring draait de projectie, de hoekgrepen schalen haar en de middelste greep bepaalt het beginpunt van deze wijzigingen.
- `Alt`-slepen met links op een tegelrand trekt schuin, net als `Shift`-slepen met de middelste knop.
- Elke sleepbeweging is één stap voor ongedaan maken, en `Esc` annuleert een sleepbeweging die bezig is.

Onder het canvas staan **Verschuiving**, **Schalen**, **Hoek** en, als het spel ze gebruikt, **Inhoudsvlaggen**, **Oppervlaktevlaggen** en **Oppervlaktewaarde**. De knoppen zijn **Herstellen**, **Terugzetten naar wereld**, **Passend maken**, **H spiegelen**, **V spiegelen** en de twee draaiingen van 90°. **Uitlijnen** draait het materiaal langs een van de randen van het vlak (druk nogmaals voor de volgende rand, `Shift` voor de vorige). De vier uitlijnknoppen duwen het materiaal tegen een rand. **U passend maken** en **V passend maken** schalen zo dat een heel aantal herhalingen het vlak bedekt; druk nogmaals voor het volgende aantal, en houd `Ctrl` ingedrukt om de onderverdeling van een trim sheet te behouden.

## UV-toetsen
Met vlakken geselecteerd verplaatsen de pijltoetsen het materiaal, `[` en `]` draaien het 15°, `,` en `.` halveren of verdubbelen de schaal, `Ctrl+F` en `Ctrl+Alt+F` spiegelen het, `Shift+R` zet het terug, `Alt+Shift+R` zet het terug naar de wereldassen en `Ctrl+Shift+F` maakt het passend op elk vlak.

**Vergrendelen** in de gereedschapsbalk behoudt de projectie als brushes worden verplaatst, en **UV** (`U`) behoudt haar als brushes draaien.

## Gereedschapsmaterialen
- skip: verwijdert het gemarkeerde vlak uit engine-exports.
- clip: maakt botsing zonder zichtbare geometrie.
- origin: markeert het draaipunt van een brush-entiteit.

De namen kun je wijzigen op de kaart **Gereedschapsmaterialen** in het exportvenster. Gebruik skip voor vlakken waarvan je weet dat ze verborgen zijn.

## Vlakken weglaten uit de export
Selecteer een vlak en zet **Vlak weglaten uit export** aan in de sectie UV. Oranje arcering markeert het in de UV-voorvertoning. Het materiaal en de verf blijven opgeslagen, en de brush blijft gesloten en bewerkbaar. Geëxporteerde render-meshes laten dat vlak weg, terwijl de botsing behouden blijft. Laat je één vierhoek van een kubus weg, dan levert dat twee renderdriehoeken op. Vlakken worden hierdoor niet dubbelzijdig en meshes worden niet samengevoegd.

**Bedekte vlakken weglaten** markeert vlakken die volledig door statische brushes worden bedekt, inclusief identieke vlakken die elkaar raken en ingesloten brushes. Het werkt op de geselecteerde brushes, of op alle zichtbare, niet-vergrendelde wereldgeometrie als er niets is geselecteerd. Brush-entiteiten, transparante materialen en brushes die al weggelaten vlakken hebben, worden overgeslagen. Gedeeltelijk bedekte vlakken worden niet gesplitst. Eén keer ongedaan maken herstelt alles. Beeld → Weggelaten vlakken tonen verbergt of toont zulke vlakken in de editorweergaven; bij het aanklikken telt nog steeds de hele brush.

# Kleur en verlopen
De sectie **Kleur** in het tabblad Vlak beschildert vlakken met egale kleuren of verlopen bovenop hun materialen. Dat is handig om een level grof op te zetten en voor gestileerde kunst. Beschilderde vlakken houden hun echte kleuren in de weergaven; de selectie wordt aangegeven met haar omtrek.

## Palet en mengkleur
Klik op een van de twaalf kleurstalen van het palet om de geselecteerde vlakken, of hele geselecteerde brushes, ermee te verven. **Dekking** bepaalt hoe dekkend de volgende verflaag is, in procenten. **Mengen** heeft een kleurknop die de kleurkiezer opent; sleep in de kiezer en de selectie volgt live, of druk op **Verven** om de mengkleur in één stap toe te passen. **Terug naar materiaal** verwijdert de verf, zodat het vlak weer zijn materiaal toont. De regel bovenaan de sectie zegt wat er geselecteerd is en wat het draagt.

## De kleurkiezer
Elke kleurknop in de editor opent dezelfde kiezer. Sleep in het vierkant om verzadiging en helderheid te kiezen, en in de strook ernaast om de tint te kiezen. Daaronder staan waarden voor **R**, **G** en **B** en een schuifregelaar **Dekking** als de kleur er een heeft. Het hexveld accepteert `#RRGGBB` of `#RRGGBBAA`; het wordt rood zolang de tekst geen geldige kleur is. **Kopiëren** zet de hexcode op het klembord. Klik buiten de kiezer om hem te sluiten.

## Verlopen
**Verloop** kiest Lineair verloop of Radiaal verloop, en de knop ernaast draait de kleuren om. Elke kleurstop heeft een kleurknop en **Positie (%)**, en stops tussen de uiteinden hebben een verwijderknop. **Kleurstop toevoegen** voegt een stop toe in het grootste gat; een verloop kan tot acht stops hebben. **Verloophoek** bepaalt de richting van een lineair verloop, met de knoppen **+45°** en **−45°**.

Onder de voorvertoning staat een rail met een markering voor elke stop. Klik op de rail om een stop toe te voegen, klik op een markering om de kiezer ervan te openen, sleep een markering om haar te verplaatsen, en klik met rechts op een markering om haar te verwijderen. De drie kleurstalen onder de rail zijn voorinstellingen: Warm verloop, Koel verloop en Grijswaardenverloop. Wijzigingen aan een verloop worden live op de selectie toegepast.

De richting van een verloop doet ertoe: leg het langs een dakhelling of langs een gebogen object, niet dwars over vlakken die niets met elkaar te maken hebben. Bekijk het model na het verven van beide kanten.

## Oppervlaktemateriaal
**Oppervlaktemateriaal** stelt **Ruwheid** in (0 is glanzend, 100 is mat), **Metallic** (0 voor hout, steen of plastic, 100 voor metaal) en **Emissie** (0 tot 8; het oppervlak gloeit in zijn eigen kleur, ook bij verlopen, maar verlicht geen objecten in de buurt). Wijzigingen worden live toegepast op de geselecteerde vlakken of brushes. **Standaardoppervlak gebruiken** verwijdert de afwijkende instelling. Alle wijzigingen kunnen ongedaan worden gemaakt.

Oppervlakte-effecten zijn te zien in de belichte 3D-weergave en in camerarenders. Vlakke weergave en draadmodel laten de hooglichten bewust weg. Bloom kan een gloed rond emitterende oppervlakken toevoegen. Godot en glTF/GLB krijgen de materiaalwaarden mee, en de Unity-importer zet ruwheid om in gladheid. Zie `docs/surface-materials.md` voor de beperkingen van elk doel.

# Assets, lagen en groepen
## De assetbibliotheek
Open het tabblad **Assets**. Voeg met **Map toevoegen** een map met `.map`-modellen toe; de ingebouwde bibliotheek is er altijd. Typ in **Assets filteren** om te zoeken. Klik op een model om het te selecteren, en dubbelklik er dan op, druk op `Enter` of klik op **Invoegen** om het toe te voegen waar de aanwijzer het laatst was. Je kunt een model ook naar een weergave slepen. Assetbrushes worden als één groep ingevoegd, zodat het model als geheel beweegt.

**Selectie als asset opslaan** schrijft de geselecteerde brushes als `.map` naar de bibliotheek. Bibliotheekassets zijn gewone bewerkbare brushes, geen geïmporteerde meshes.

## Lagen
De lijst **Lagen** in het tabblad Map ordent grote gebieden. Typ een naam en klik op **Toevoegen** om een laag te maken, of druk op `Enter` om de huidige laag te hernoemen. Elke laag kan worden verborgen, vergrendeld of uit exports weggelaten. **Hierheen verplaatsen** verplaatst de selectie naar de huidige laag. Het contextmenu in de weergaven biedt ook Naar laag verplaatsen, Actieve laag maken, Lagen verbergen en Lagen isoleren.

## Groepen en gekoppelde groepen
`Ctrl+G` groepeert de selectie en `Ctrl+Shift+G` heft de groepering op. De sectie **Groepen** in het tabblad Map toont de groepen en biedt **Openen**, **Hernoemen**, **Samenvoegen**, **Groepering opheffen**, **Eruit halen** en **Losmaken**. Een gekoppelde groep is een set kopieën die bewerkingen delen: `Ctrl+Shift+D` maakt een gekoppeld duplicaat, Geometrie → Gekoppelde groepen synchroniseren werkt de kopieën bij, en **Ontkoppelen** maakt een kopie zelfstandig. In het tabblad Entiteit kan een eigenschap als eigen waarde van één kopie behouden blijven wanneer gekoppelde groepen worden gesynchroniseerd.

## Een gedeelde schaal
De standaard exportschaal is 32 mapeenheden per meter. De ingebouwde markering voor de startpositie van de speler is 64 eenheden hoog, een referentie van 2 meter. Vergelijk deuropeningen, voertuigen, meubels en planten ermee. Verander de exportschaal alleen bewust en houd samenhangende assets consistent.

# Entiteiten en gameplay
Veelgebruikte entiteitsdefinities zijn ingebouwd en altijd beschikbaar: lampen, startposities van de speler, deuren, knoppen, platforms, triggers, statische props, geluiden en waypoints. Laad je een FGD van je project met **Entiteitsdefinities** in het tabblad Entiteit, dan vervangt die klassen met dezelfde naam en blijven de overige ingebouwde klassen behouden.

## Entiteiten plaatsen
Typ in de browser **Entiteiten** van het tabblad Entiteit in **Klassen filteren** en kies een klasse. Voor een puntklasse zoals een lamp klik je op **Plaatsen** of dubbelklik je op de klasse; `Insert` plaatst er een in het midden van de selectie. Voor een brushklasse zoals een deur of trigger selecteer je de brushes en klik je op **Maken**. Klassen kun je ook naar een weergave slepen.

## Eigenschappen
De eigenschappenlijst bewerkt de geselecteerde entiteit. Entiteiten van dezelfde klasse worden samen bewerkt. **Eigenschap toevoegen** voegt een sleutel toe die de definitie misschien niet kent; **Standaardwaarden** kan bestaande eigenschappen terugzetten naar hun standaardwaarde of de ontbrekende toevoegen. Eigenschappen die de klasse definieert maar die de entiteit niet heeft, worden gedimd getoond; typ een waarde om ze toe te voegen. Gebruik targetname en target om entiteiten te koppelen.

## Wat een export kan doen
De directieven van Godot maken native nodes, botsingslichamen en triggergebieden. Unity en Unreal ondersteunen gangbare lampen native, en Unreal maakt ook PlayerStart-actors. Overige gegevens blijven behouden voor zover de importer van het doel dat toelaat.

Een deurentiteit alleen voegt geen spelgedrag toe. Beweging, reacties op triggers, spelerbesturing en projectscripts vragen nog steeds werk in de engine. Defold exporteert entiteitseigenschappen en botsingsobjecten; Flax plaatst modellen en generieke entiteiten. Controleer het resultaat in je engine voordat je er gameplay op bouwt.

# Animatie
Heron Brush kan eenvoudige beweging voorvertonen en keyframe-animaties van brush-entiteiten bewerken, en die vervolgens naar verschillende engines exporteren.

## Voorvertoning van animaties
Zet **Voorvertoning van animaties** aan in het menu Beeld, het menu Animatie, het menu Rasterinstellingen of de sectie Beeld van het tabblad Map. Het speelt de rotatie van func_rotate en de heen-en-weerbeweging van func_shuttle af als start_on aanstaat, met de eigenschappen axis, speed, direction, travel en wait van de entiteit, en het speelt keyframe-clips af die op Afspelen bij start staan. De assets van de ventilator en de windmolen bevatten al draaiende onderdelen.

Zet je de voorvertoning uit, dan keert de oorspronkelijke houding terug. Een klik in een weergave stopt het afspelen voordat er iets wordt geselecteerd of bewerkt; camera-navigatie blijft werken. De voorvertoning verandert de map, de geschiedenis voor ongedaan maken en de exports niet, en voert geen fysica of spellogica uit.

## De keyframe-editor
Open hem met Animatie → Keyframe-animatie of vanuit het menu Rasterinstellingen. Hij wordt onder de weergaven gedockt, of in het 3D-venster zolang dat apart staat. Het vensterpictogram in zijn kop verplaatst hem naar een eigen venster; het frame en de geschiedenis voor ongedaan maken blijven behouden. **Animatie bewerken** zet de weergaven in de geanimeerde houding en **Map bewerken** gaat terug naar de rustgeometrie. In animatiemodus toont elke weergave onderaan **Animatie bewerken** en het framenummer.

Selecteer een brush-entiteit, of een brush of brushgroep, om die te animeren. Een platte brushgroep deelt één animatie en één draaipunt; groepen met meerdere entiteiten hebben afzonderlijke animaties nodig.

## Clips
De keuzelijst **Animatie** wisselt tussen benoemde clips. Typ een naam in **Animatienaam** en klik op **Opslaan als** om de huidige clip onder die naam te kopiëren; **Verwijderen** verwijdert een clip. **Verplaatsen** maakt een clip `translate` zonder rotatie. **Deuranimaties** maakt vloeiende clips `open` en `close` met een draai van 90 graden om het huidige draaipunt; verplaats het draaipunt eerst naar het scharnier. **Afspelen bij start** laat de clip afspelen wanneer het level start; de deurvoorinstelling zet dit uit.

## Transport en afspelen
De transportknoppen zijn **Start**, **Vorige sleutel**, **Afspelen**, **Volgende sleutel** en **Einde**. **Frame** toont en stelt het huidige frame in, en **Duur (s)** stelt de lengte van de clip in. De modus is **Eenmaal**, **Lus** of **Pingpong**.

## De tijdlijn
De tijdlijn heeft een liniaal met de afspeelkop en een baan voor **Positie** (X, Y en Z samen als sleutel) en voor elke rotatiehoek, RX, RY en RZ. Klik of sleep in de liniaal om de afspeelkop te verplaatsen en die houding in elke weergave te bekijken. Sleep met de middelste knop om de tijdlijn te pannen. Sleep sleutels om ze in de tijd te verplaatsen; `Shift` voegt toe aan de selectie; dubbelklik op een baan om een sleutel in te voegen; `Ctrl`+muiswiel zoomt. **Curven** toont de waardecurven in plaats van de banen.

De knoppen erboven zijn **Sleutel toevoegen**, **Verwijderen**, **Kopiëren**, **Plakken** en **Dupliceren**. **Aan frames kleven** houdt sleutels op hele frames en **Passend maken** toont de hele clip. **Sleutels hertimen** opent **Tijdschaal**, **Verschuiving (frames)**, **Schalen vanaf afspeelkop**, **Duur verlengen** en **Selectie hertimen**.

De sleutelinspector naast de tijdlijn bewerkt de geselecteerde sleutel: **Frame**, **Waarde** (of de drie coördinaten van een positiesleutel), de interpolatie **Lineair**, **Constant** of **Bézier**, **Inkomende helling** en **Uitgaande helling**, en **Vloeiende raaklijnen**.

## Toetsen in de editor
Zolang de tijdlijn het toetsenbord heeft: `Space` speelt af of pauzeert, de pijl naar links en naar rechts gaan één frame verder, `Ctrl+A` selecteert alle sleutels, `Ctrl+C`, `Ctrl+X` en `Ctrl+V` kopiëren, knippen en plakken sleutels, `Ctrl+D` dupliceert ze, `Delete` verwijdert ze, `Ctrl+Z` maakt ongedaan en `Esc` annuleert een sleepbeweging. Buiten de tijdlijn verlaat `Esc` de animatiemodus. Ongedaan maken laat de zoom, de selectie en de afspeelkop staan waar ze waren.

## Poseren in de weergaven
De rij onder de tijdlijn bevat **Onion skinning** met een aantal en **Tussenruimte**, dat nabije houdingen als schimmen toont; de coördinaten van het **Draaipunt**; **Automatische sleutels**; en de posegereedschappen **Verplaatsen**, **Roteren** en **Draaipunt**.

- **Sleutel toevoegen** zet Automatische sleutels aan en pauzeert het afspelen. Met Automatische sleutels aan en **Verplaatsen** gekozen sleep je een gekleurde as van de handgreep van de geselecteerde entiteit in de 3D-weergave om de positie op het huidige frame vast te leggen.
- Sleep in een 2D-weergave de geselecteerde brush of een lid van zijn groep om een verplaatsing langs de twee zichtbare assen vast te leggen, met kleven aan het raster. Automatische sleutels hoeft hiervoor niet aan te staan.
- Sleep met **Roteren** de brush in een 2D-weergave om zijn oranje draaipuntmarkering. Bovenkant legt RZ vast, Voorkant RY en Zijkant RX, gekleefd aan rasterhoeken.
- Sleep met **Draaipunt** het gele rotatiemiddelpunt in een willekeurige 2D-weergave. Het kleeft aan het raster en behoudt zijn diepte. Sleutels en rustgeometrie blijven ongewijzigd.

Het cyaan pad in 2D en 3D toont de baan van het draaipunt. Klik op een punt van het pad om de tijd en de sleutels ervan te selecteren, of sleep het om de positie te veranderen; met Automatische sleutels aan kun je padpunten ook in de 3D-weergave verslepen. Labels tonen framenummers en gestreepte segmenten markeren sprongen. `Esc` annuleert elk van deze sleepbewegingen, en één keer ongedaan maken draait een hele sleepbeweging terug.

Nieuwe animaties beginnen in rust met sleutels op tijdstip nul. Maak beweging door de houding op een ander frame te veranderen. Voor een scherp L-vormig pad leg je het begin, de hoek en het einde vast en gebruik je Lineair met Pingpong.

## Knoppen aan animaties koppelen
Selecteer een func_button, trigger_multiple of trigger_once. Het tabblad Entiteit toont **Animatietrigger** boven de eigenschappen. Kies een doel uit de lijst, of klik op **Doel aanwijzen** en daarna op een geanimeerd object in een willekeurige weergave; `Esc` annuleert het aanwijzen. Ontbrekende doelen of clips worden onder de bedieningselementen gemeld. Entiteitskoppelingen tonen pijlen naar hun doelen als de weergave van koppelingen aanstaat.

**Actie** kiest wat activering doet: **Afspelen** start een benoemde clip of de geselecteerde animatie; **Omschakelen** wisselt tussen `open` en `close` als die bestaan, en start anders de geselecteerde animatie opnieuw; **Openen** en **Sluiten** starten die clips; **Pauzeren** houdt de houding vast, **Hervatten** gaat verder en **Stoppen** herstelt de oorspronkelijke houding. **Slechts eenmaal** overschrijft de standaardwaarde van de klasse. **Herhaalvertraging (s)** stelt de kortste tijd tussen activeringen in; de eerste wordt er niet door vertraagd.

**Trigger testen** voert de actie uit in de voorvertoning zonder de map te bewerken of stappen voor ongedaan maken toe te voegen. **Voorvertoning herstellen** herstelt de oorspronkelijke houdingen en wist de eenmaal- en afkoelstatus. Botsingen met de speler en spelscripts worden hiermee niet gesimuleerd.

## Animatie in exports
Godot exporteert native AnimationPlayer- en AnimationLibrary-resources, inclusief curven en bewegende botsing. Een CharacterBody3D die het gebied van een knop binnengaat, activeert het doel ervan. Wijzigingen die je in Godot aan deze clips maakt, komen niet terug in de map en worden bij de volgende export vervangen.

De raylib-export voegt `<map>_animation.h` toe met een speler in C en `<map>_preview.c` met een voorvertoning met vrije camera; volg `<map>_raylib.md` om hem te bouwen. In de voorvertoning pauzeert `SPACE`, start `R` de standaardclips opnieuw en loopt `N` door de benoemde clips. Defold bouwt het afspelen in het entiteitsscript in, met gedocumenteerde berichten `heron_*_animation`. Build Level van Unity voegt `HeronBrushAnimation` en een kinematisch lichaam toe. De Bevy-export bevat `heron_animation.rs` en instructies. Godot, Unity, Defold, Bevy en raylib behouden de triggerkoppelingen, de eenmaal-vlag en de herhaalvertraging. Je spel moet de speler nog steeds registreren en zijn eigen knoppen aan de API van elke runtime koppelen.

## Het voorbeeld Motion lab
Open `motion_lab` via Voorbeelden op het welkomstscherm. Het bevat een platform met een L-vormig pad, een lift, een draaiend platform en een scharnierende deur met knoppen. Open het `godot/project.godot` ernaast in Godot en druk op `F5`. WASD en de muis bewegen de speler, Space springt, 1 tot 4 bezoeken de stations en R start het huidige station opnieuw. Stap op een gekleurde plaat om het doel ervan te activeren. De bijgeleverde README legt uit hoe je opnieuw exporteert.

# Exporteren naar een engine
Bestand → Exporteren opent het exportvenster. De engines staan links onder elkaar met hun logo's: Godot, Defold, three.js, Bevy, raylib, Unity, Unreal Engine, Flax Engine, Cocos Creator, Wavefront .obj, glTF .gltf, GLB .glb en als laatste `.map`. De pagina rechts zegt wat het doel krijgt. **Schrijven** schrijft het, **Controleren** bouwt het level zonder te schrijven en somt de problemen op, en **Sluiten** sluit het venster.

Exporteren staat los van opslaan: de `.map` blijft je bewerkbare bron. Elk bedieningselement slaat de instellingen van zijn engine op zodra het verandert, zodat elke engine zijn eigen antwoorden houdt en het venster opent zoals je het hebt achtergelaten.

## Gemeenschappelijke opties
- **Schalen**: **Eenheden per meter**, standaard 32.
- **Wat er geschreven wordt**: **Botsingsgeometrie**.
- **Optimalisaties**: **Statische meshes groeperen in cellen van 1024 eenheden**, **Vlakken weglaten die in de geometrie verborgen liggen**, **Vlakken in hetzelfde vlak samenvoegen** en **Botsingsvormen samenvoegen**.
- **Projectpaden**: waar het level en zijn resources binnen het project terechtkomen.
- **Gereedschapsmaterialen**: de namen van de materialen clip, skip en origin.
- **Meer**: engine-specifieke schakelaars zoals **Nodes benoemen naar**, **Lagen en groepen opnieuw opbouwen als bovenliggende nodes**, **Draairichting van de driehoeken omkeren**, het spiegelen van texturecoördinaten en een tweede UV-set voor gebakken licht met **Lightmap-texel**.
- **Bijhouden met de map**: **Opnieuw schrijven telkens als de map wordt opgeslagen**. Dit wordt beschikbaar nadat de engine één keer is geëxporteerd, omdat die export bepaalt waar er geschreven wordt.

Statische cellen wijzen wereldbrushes toe op basis van hun middelpunt en combineren ze per materiaal binnen elke cel van 1024 eenheden, zodat engines kleinere delen van de map kunnen wegfilteren (cullen). Botsing blijft behouden en gameplay-entiteiten blijven gescheiden. Het kan meer draw calls opleveren dan één wereldmesh, daarom staat het standaard uit. Losse OBJ-, glTF- en GLB-exports gebruiken geen cellen.

## Opmerkingen per doel
- Godot: `res://`-paden voor textures en materialen, de fysicalaag voor triggers en lightmap-UV2. Kies een exportmap binnen het Godot-project.
- Defold: de resourcemap voor assets, het modelmateriaal en de texturesampler, die in het gekozen materiaal moet bestaan.
- Unity: OBJ-meshes en een editor-importer; exporteer onder de map Assets van het project.
- Unreal Engine: een gegenereerd importscript en een contentpad, normaal /Game/<level>.
- Flax Engine: een contentpad voor modellen, relatief aan de map Content van het project.
- three.js, Bevy, raylib en Cocos Creator: scènes of code voor die runtimes, met instructies naast de export.
- glTF / GLB: overdraagbare geometrie voor compatibele tools; GLB is de binaire vorm. OBJ: geometrie en materialen voor modelleerprogramma's.
- `.map`: **Schrijven als** kiest Valve 220, Standaard, Quake 2 of Quake 2 (Valve). Het venster zegt wat het gekozen dialect zou verliezen.

Lees na het schrijven de waarschuwingen, bijvoorbeeld over ontbrekende textures. **Wat de controle heeft gevonden** somt ze op in het venster.

# Livekoppelingen: Godot, Unity, Defold en Flax
Een livekoppeling stuurt niet-opgeslagen bewerkingen terwijl je werkt vanuit Heron Brush naar een engine-editor. Ze werkt in één richting: bewerkingen in de engine komen niet terug. Per mapvenster kan één livekoppeling draaien. Stop haar voordat je van engine wisselt, en start haar opnieuw om gewijzigde exportopties toe te passen. De bedieningselementen staan in het exportvenster, op de pagina van de engine, op de kaart **Livekoppeling met …**.

De indicator in de titelbalk toont de status van de koppeling; klik erop om de koppeling te stoppen. Het sluiten of vervangen van de map stopt het publiceren. Na een mislukte import los je het probleem op en start je de koppeling opnieuw.

## Godot
1. Exporteer één keer naar een Godot-project.
2. Klik op **Godot-add-on installeren**.
3. Schakel Heron Brush Live in via Project Settings → Plugins in Godot.
4. Klik op **Livekoppeling starten** en koppel de map vanuit het Godot-dock aan een open 3D-scène.

De add-on vervangt alleen zijn gegenereerde tak en behoudt de plaatsing van de koppeling en andere nodes. Sla de Godot-scène zelf op, en houd je eigen scripts en nodes buiten de gegenereerde tak.

## Unity
1. Exporteer één keer onder de map Assets van het Unity-project.
2. Klik op **Unity-add-on installeren** en laat Unity hem compileren.
3. Open in Unity Window → Heron Brush → Live Link.
4. Start de koppeling en koppel de map aan de actieve Unity-scène.

Houd het livevenster van Unity open. Updates pauzeren in Play Mode en Prefab Mode. Alleen het kind Generated van het mapobject wordt vervangen. Bewerk de gegenereerde bestanden niet.

## Defold
1. Exporteer één keer naar je Defold-project en houd het project open in Defold.
2. Klik op **Livekoppeling starten**. Er is geen add-on nodig.
3. Open heron_brush_live/preview.collection in Defold, of verwijs ernaar vanuit je eigen scène.

De koppeling schrijft naar een aparte gegenereerde map en laat de uiteindelijke export en game.project ongemoeid. **Draaiend spel live herladen** werkt ook een spel bij dat vanuit Defold is gestart; voor het toevoegen van objecten of het wijzigen van botsing kan het nodig zijn het level opnieuw te starten. Mislukte verbindingen worden automatisch opnieuw geprobeerd.

## Flax
Exporteer naar de map Content van het project en klik op **Flax-add-on installeren**. Laat Flax hem compileren, start de koppeling en kies dan Tools → Heron Brush: attach live map in de open Flax-scène. De gegenereerde root behoudt zijn plaatsing en andere objecten blijven onaangeroerd. Sla de scène zelf op. Updates pauzeren in Play Mode. Tools → Heron Brush: stop live map pauzeert de ontvangst.

# Samenwerken in livesessies
Samenwerking → Livesessie opent een venster om een sessie te hosten of deel te nemen aan die van een andere Heron Brush-gebruiker. Zo bewerken meerdere mensen samen een map; dit staat los van een livekoppeling met een engine.

## Hosten en deelnemen
Vul **Je naam** in. Om te hosten kies je **Een sessie hosten**, klik je op **Sessie starten** en stuur je de gegevens van **Verbindingsgegevens kopiëren** naar je gasten. Om deel te nemen kies je **Deelnemen aan een sessie**, plak je de gegevens en klik je op **Deelnemen**. Deelnemen vervangt de open map door de map van de host, dus sla eerst je werk op.

De standaardversie werkt in het lokale netwerk, en **Lokale netwerkpoort** stelt de poort in. Sessies via internet (**Via internet verbinden**) vereisen een versie met de optionele netwerkfunctie. Deelnemers zien elkaars cameramarkeringen en namen in de 3D-weergave.

Het sluiten van het venster laat de sessie doorlopen; de indicator in de titelbalk toont de status. **Sessie stoppen** of **Sessie verlaten** beëindigt haar. Bewerken twee mensen hetzelfde object, dan bepaalt de volgorde van de host de uiteindelijke versie, en ook ongedaan maken is een gedeelde bewerking. Werk waar mogelijk aan verschillende gebieden en sla tussentijds op.

# 3D-weergave en een tweede monitor
## Weergavemodi
De sectie **Beeld** in het tabblad Map bepaalt wat de weergaven tekenen: Brushes, Puntentiteiten, Entiteitsmodellen, Klassenamen van entiteiten, Groepscontouren, Patches, Materialen, Weggelaten vlakken tonen, Voorvertoning van animaties, Draadmodel, Randen en Contouren van brush-entiteiten. Draadmodel tekent de randen van driehoeken, inclusief diagonalen en randen achter oppervlakken, zonder materialen. Het is een tijdelijke debugweergave en heeft geen invloed op exports of renders.

Belichting, schaduwen, ambient occlusion, reflecties, bloom en gladmaken voor de 3D-weergave staan in Voorkeuren → Renderer. Dit zijn instellingen voor de voorvertoning; eindbeelden gebruiken de eigen instellingen van de rendercamera.

## Een apart 3D-venster
Beeld → 3D-weergave in apart venster verplaatst de 3D-weergave naar een eigen venster, dat je op een andere monitor kunt zetten. Het deelt de camera, het document en de gereedschappen, en de platte weergaven nemen de ruimte in het hoofdvenster in. Het menu biedt Materialen, Massieve weergave en Draadmodel; een submenu Beeld met Weggelaten vlakken tonen, Voorvertoning van animaties, Randen, Groepscontouren, Puntentiteiten, Entiteitsmodellen en Contouren van brush-entiteiten; Op selectie focussen; Voorkant, Zijkant en Bovenkant, die de camera rond de selectie draaien (of rond een punt voor de camera); Camera plaatsen vanuit 3D-weergave; Volledig scherm (`F11`); Afbeelding renderen; en Terug naar hoofdvenster. Sluit je het venster, dan wordt de weergave weer gedockt.

## Procedurele lucht
De procedurele lucht staat in de 3D-weergave standaard aan en kan worden uitgezet in Voorkeuren → Renderer → Licht. Rendercamera's hebben hun eigen schakelaar **Procedurele lucht**, die met de map wordt opgeslagen. De kleuren van lucht, horizon en grond komen uit de omgevingsinstellingen, en de zon volgt de belichting van de scène. De lucht heeft geen geometrie en wordt niet geëxporteerd.

# Rendercamera's en PNG-afbeeldingen
Bestand → Afbeelding renderen opent het rendervenster: links de camera en haar instellingen en rechts een live voorvertoning die opnieuw wordt getekend terwijl je ze wijzigt. Camera's worden in de `.map` opgeslagen in een laag Render cameras die alleen voor de editor is en uit engine-exports wordt weggelaten.

## De camera plaatsen
Kader het beeld in de 3D-weergave en druk op `Ctrl+Alt+0` (of `Ctrl+Alt+Num0`), of klik op **Camera plaatsen vanuit 3D-weergave**. Is er geen camera geselecteerd, dan wordt de eerste opgeslagen camera bijgewerkt; bestaat er geen, dan wordt er een gemaakt. Dit kan ongedaan worden gemaakt en behoudt de instellingen voor uitvoer en belichting. **Camera in map selecteren**, **Camera uit map laden** en **3D-weergave naar camera verplaatsen** werken andersom. **Cameraframe in 3D-weergave** toont de grens van de uitvoer, en met **Camera vanuit 3D-weergave besturen** verplaatst de normale navigatie de opgeslagen camera.

## Instellingen
- **Camerapositie**: X, Y, Z, yaw en pitch.
- **Uitvoer en lens**: **Voorinstellingen** (Full HD, Vierkant, Staand), breedte, hoogte, samples, **Verticaal gezichtsveld** en **Verre knipgrens**.
- **Belichting**: Daglicht, Zonsondergang of Bewolkt, of stel zelf de belichtingswaarde en de kleuren van zon, lucht, horizon en grond in, plus **Procedurele lucht**, **Schaduwen**, **Ambient occlusion** en **Bloom**.
- **Scherptediepte**: **Scherptediepte inschakelen**, scherpstelafstand, scherptebereik en vervagingsstraal. **Op selectie focussen** stelt scherp op het midden van een geselecteerd object voor de camera.

**Camera en instellingen opslaan** bewaart het formulier in de map; sla de map op om het op schijf te bewaren.

## Renderen
**PNG renderen** vraagt waar je wilt opslaan en start een progressieve render, die zich in de voorvertoning sample voor sample opbouwt. Meer samples zien er beter uit maar duren langer. **Render annuleren** stopt zonder een bestand te schrijven. De renderer gebruikt de rasterpipeline van de editor, geen path tracing. De PNG heeft een ondoorzichtige achtergrond en animaties worden niet gerenderd. Zeer grote afbeeldingen kunnen de editor even laten haperen.

# Voorkeuren
Bestand → Voorkeuren (`Ctrl+,`) opent de instellingen. Wijzigingen worden direct toegepast en opgeslagen. De zijbalk toont Overzicht, Spellen, Beeld, Kleuren, Muis, Toetsenbord en Renderer. Het zoekveld bovenaan vindt een instelling op naam of beschrijving op elke pagina. Elke gewijzigde instelling heeft een eigen knop om de standaardwaarde terug te zetten, en **Standaardwaarden herstellen** zet de huidige pagina terug.

## De pagina's
- **Overzicht**: de gangbare instellingen bij elkaar: Uiterlijk (Dekking van panelen (%), Kleurenschema, Indeling, Taal), Bewerken, Beeld en Handleiding.
- **Spellen**: het spel, waar de bestanden ervan staan, en je naam voor samenwerking. Een spel bepaalt welk mapformaat wordt geschreven, welke entiteitsdefinities worden geladen en waar de materialen staan. Spelconfiguraties worden gelezen uit de map `games` in de configuratiemap (`~/.config/heron-brush/games` op Linux en macOS, `%LOCALAPPDATA%\heron-brush\games` op Windows) en uit de map `games` naast de toepassing, waar pakketten de meegeleverde configuratie neerzetten.
- **Beeld**: Indeling, Kleurenschema, Taal, wereldassen, dekking van het raster, celgrootte voor de lekcontrole en de projectievergrendelingen.
- **Kleuren**: de kleuren van de weergaven, de geometrie en de handgrepen.
- **Muis**: snelheden en omkeringen voor kijken, pannen en verplaatsen, vliegsnelheid, gezichtsveld, 2D-weergaven koppelen en de grootte van handgrepen.
- **Toetsenbord**: de sneltoetsen, Heron Brush of Blender.
- **Renderer**: realistische schaduwering, selectiecontour, licht, schaduwen, ambient occlusion, reflecties, bloom en oppervlakken.

## Taal
**Taal** biedt Bureaublad volgen en elke vertaalde taal. De interface verandert meteen. De handleiding opent in de taal van de interface als er een vertaling van bestaat, en anders in het Engels.

# Plug-ins en MCP
## Plug-ins
Plug-ins breiden de editor uit met Rhai-scripts. Bestand → Plug-ins opent het venster Plug-ins. **Een plug-in installeren** installeert een `.rhai`-bestand en **Een plug-inmap installeren** installeert een map met `main.rhai`; optionele metadata in `plugin.toml` beschrijft de plug-in en de instellingen ervan. Plug-ins zijn gegroepeerd onder **Actieve plug-ins** en **Inactieve plug-ins**, en **Plug-in inschakelen** zet er een aan of uit; dit blijft na een herstart behouden. **Plug-ins opnieuw laden** leest de scripts opnieuw van schijf en zet hun paneelwaarden terug.

Actieve plug-ins verschijnen ook in de sectie **Plug-ins** van het tabblad Map, met hun instellingen, actieknoppen (**Uitvoeren**) en uitvoer. Een actie werkt op de selectie, of op alles als er niets is geselecteerd, en is één stap voor ongedaan maken. De meegeleverde Selection inspector toont aantallen en acties om omhoog en omlaag te verplaatsen.

## MCP
Samenwerking → Aanbieden voor een agent (MCP) laat een AI-agent of een ander programma de open editor besturen via het Model Context Protocol. Het venster toont het lokale adres met **Kopiëren**, en **Server stoppen** beëindigt het; zolang het draait, toont de titelbalk **MCP actief**. De opdrachtregeloptie `--mcp` draait dezelfde interface zonder venster. Clients kunnen gereedschappen ontdekken voor geometrie, assets, eigenschappen, animatie, export en sessies. MCP en editor-plug-ins zijn verschillende interfaces.

# Opdrachtregel
Het programma heet `heron-brush`. Zonder opties opent het het welkomstscherm.

## Bestanden openen
`heron-brush map.map [entities.fgd] [materials/]` opent een map, eventueel met entiteitsdefinities en een materiaalmap. Een leeg argument slaat er een over, bijvoorbeeld `heron-brush map.map "" materials/`. Draait er al een editor, dan opent de map daarin; `--new-instance` opent in plaats daarvan een apart venster.

## Opdrachten
- `--help` of `-h`: het gebruik tonen. `--version` of `-V`: de versie tonen.
- `--mcp`: MCP aanbieden via standaardinvoer en -uitvoer, zonder venster.
- `--merge-map <base> <ours> <theirs> [<output>]`: een driewegsamenvoeging van `.map`-bestanden voor git. De uitvoer is standaard ours. Conflicten worden afgedrukt en leveren een afsluitcode anders dan nul op.
- `--obj-to-map <in.obj> <out.map> [size] [thickness] [snap] [material]`: een mesh omzetten in brushes. Size is de lengte van de langste zijde in mapeenheden (standaard 256), snap is een raster voor de hoekpunten (0 voor geen), en de woorden `solid` of `layered` bouwen rasterblokken of convexe platen in plaats van dunne wiggen.
- `--render-preview <in.map> <out.png> [size]`: een PNG-miniatuur van een map schrijven, standaard 512 pixels.
- `--run-script <script.rhai> <in.map> [out.map] [key=value ...]`: een plug-inscript op een map uitvoeren met alles geselecteerd en het resultaat schrijven (standaard terug naar de invoer). `key=value` beantwoordt de instellingen van de plug-in.

# Problemen oplossen
## Als er iets niet klopt
- Ontbrekend materiaal: controleer de materiaalmap of het pakket en de projectpaden van de export.
- Verkeerde modelgrootte: vergelijk met de spelermarkering van 64 eenheden en de exportschaal.
- Verkeerde richting van het verloop: controleer de verloophoek en de oriëntatie van het vlak.
- Lege scène in de engine: lees de exportwaarschuwingen en volg de importinstructies van het doel.
- Livekoppeling wacht: controleer of de add-on is geïnstalleerd, de scène is gekoppeld en de updates niet zijn gepauzeerd.
- Alleen lijnen, geen vlakken: zet Draadmodel uit in de sectie Beeld van het tabblad Map, of kies Materialen of Massieve weergave in het menu van het 3D-venster.
- Objecten ontbreken in de weergaven: `Ctrl+Shift+I` toont verborgen objecten; controleer de lagen en de weergavefilters.
- Een toets doet niets: klik eerst in de weergave zodat die het toetsenbord heeft, en controleer de sneltoetsen in Voorkeuren → Toetsenbord.
- Een bewerking is mislukt: maak haar ongedaan, vereenvoudig de geometrie en gebruik **Map controleren** en **Op lekken controleren** in de sectie **Problemen** van het tabblad Map.
- Een map opent niet vanuit de bestandsbeheerder: misschien is hij doorgegeven aan een editor die al draait; zoek dat venster.

## Instellingen en back-ups
Voorkeuren, recente maps, sterren, exportinstellingen en plug-ins staan in de configuratiemap: `~/.config/heron-brush` op Linux en macOS, en `%LOCALAPPDATA%\heron-brush` op Windows. Autosave-back-ups staan in de map `autosave` naast elke map.

## Een bug melden
Vermeld de versie van de editor (Help → Over), je besturingssysteem, een kleine map die het probleem laat zien en de exacte stappen. Het uitvoerpaneel (`Ctrl+4`) zegt vaak wat er misging. Maak een kopie voordat je gaat experimenteren.

## Deze handleiding gebruiken
Kies links een hoofdstuk in de inhoudsopgave. `Ctrl+F` toont de inhoudsopgave en gaat naar het zoekveld; `Esc` wist de zoekopdracht, of sluit de handleiding als de zoekopdracht leeg is. **Inhoud** verbergt of toont de lijst met hoofdstukken en **Tekstgrootte** verandert de lettergrootte; beide worden onthouden. De taallijst bovenaan wisselt tussen de beschikbare vertalingen.
