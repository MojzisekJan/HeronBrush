# Kom godt i gang
Heron Brush er en brush-baseret baneeditor. Du bygger baner af konvekse brushes, maler og teksturerer deres flader, placerer entiteter, animerer døre og platforme og eksporterer resultatet til en spilmotor. Den har også indbyggede assets, parametriske former, live-forbindelser til motorernes editorer, fælles redigeringssessioner og rendering af stillbilleder. Denne manual følger med programmet og virker uden internet.

På macOS virker `Cmd` overalt, hvor denne manual skriver `Ctrl`, og Option er Alt.

## Velkomstskærmen
Når Heron Brush starter uden et kort, åbner velkomstskærmen. Til venstre er **Fortsæt** (det sidste kort, du havde åbent), **Nyt kort** og **Gennemse** til at åbne en eksisterende `.map`, samt **Spil**, **Manual** og **Indstillinger**. Til højre er en liste over seneste kort med miniaturer, efterfulgt af **Eksempler**. Stjernen (**Hold øverst**) fastgør et kort øverst på listen; skraldespanden (**Flyt til papirkurven**) flytter filen til systemets papirkurv, så den kan gendannes. Piletasterne flytter rundt på skærmen, og Enter åbner det element, der har fokus. Fil → Velkommen henter skærmen frem igen når som helst.

## Dit første kort
1. Vælg **Nyt kort** på velkomstskærmen, eller Fil → Nyt kort (`Ctrl+N`).
2. Vælg Brush-værktøjet (`B`) i værktøjslinjen, og træk i en 2D-visning for at tegne den valgte form. Kontrollér dens dybde i en anden 2D-visning.
3. Markér brushen, og flyt den ved at trække. Brug fanen **Flade** til at give den et materiale eller en farve.
4. Gem kildekortet med `Ctrl+S`. Når du er klar til at bruge det i et spil, så åbn Fil → Eksportér, og skriv en scene til din motor.

## Filer og gemning
`.map`-filen er din redigerbare kilde. Gem jævnligt: eksport, live-forbindelser og rendering gemmer aldrig kortet for dig. Fil → Gem som (`Ctrl+Shift+S`) tilføjer endelsen `.map`, hvis du udelader den. Fil → Vend tilbage til gemt kort kasserer ikke-gemte ændringer og indlæser filen igen. Hvis du lukker et vindue eller åbner et andet kort med ikke-gemte ændringer, spørger Heron Brush, om du vil **Gem**, **Kassér** eller **Annuller**.

Mens et gemt kort har ikke-gemte ændringer, skrives nummererede sikkerhedskopier hvert tiende minut til en `autosave`-mappe ved siden af kortet. De 50 nyeste sikkerhedskopier beholdes.

Hvis du åbner et kort, mens Heron Brush allerede kører, overdrages det til det kørende vindue, som kommer i forgrunden. Brug `--new-instance` på kommandolinjen, når du vil have et separat vindue (se Kommandolinje).

# Brugerfladen
Hovedvinduet har en titellinje med menuerne, en værktøjslinje under den, kortvisningerne, inspektøren til højre og en statuslinje langs bunden.

## Titellinje og menuer
Titellinjen viser logoet, menuerne, kortets navn i midten og vinduesknapperne. På macOS sidder de sædvanlige røde, gule og grønne knapper til venstre. Træk i titellinjen for at flytte vinduet, og dobbeltklik på den for at maksimere. Mens noget kører i baggrunden, vises en indikator nær enden af linjen: en live-forbindelse til en motor, en live-session eller **MCP aktiv**. Klik på en indikator for at stoppe live-forbindelsen eller åbne dens vindue.

- **Fil**: Velkommen, Nyt kort (`Ctrl+N`), Åbn kort (`Ctrl+O`), Gem (`Ctrl+S`), Gem som (`Ctrl+Shift+S`), Vend tilbage til gemt kort, Eksportér, Render billede, Plugins og Indstillinger (`Ctrl+,`).
- **Rediger**: Fortryd (`Ctrl+Z`), Annuller fortryd (`Ctrl+Shift+Z`), Klip ud, Kopiér, Indsæt, Duplikér (`Ctrl+D`), Slet, Markér alt (`Ctrl+A`), Gentag seneste kommandoer (`Ctrl+R`) og Glem gentagelige kommandoer.
- **Visning**: 3D-visning i separat vindue, Gå til, Fastgør til gitter (`Alt+0`), Vis gitter (`0`), Forhåndsvis animationer og Vis udeladte flader.
- **Geometri**: Indsæt 3D-tekst, Sammenføj hjørne, Konvertér flader til patches, Flyt markeringen ud af dens entitet og Synkronisér sammenkædede grupper.
- **Animation**: Keyframe-animation, Forhåndsvis animationer og Placér kamera fra 3D-visning (`Ctrl+Alt+0`).
- **Kort**: Indlæs, Genindlæs og Fjern punktfil; Indlæs, Genindlæs og Fjern portalfil. De viser lækagespor og portaler, som kortkompilere har skrevet.
- **Samarbejde**: Live-session og Server for en agent (MCP).
- **Hjælp**: Brugervejledning og Om.

## Værktøjslinjen
Værktøjslinjen rummer alle værktøjer. Den første gruppe bruges hele tiden: Markér, Brush, Klip, Hjørnepunkt, Kant, Flade, Rotér og Skalér. Den anden gruppe rummer Forskyd, Saml, Fej, Kontrolpunkt, Afrunding og Mal. Hold markøren over en knap for at se dens navn og genvej. Klikker du på det aktive værktøj igen, vender du tilbage til Markér.

Efter værktøjerne kommer **Spejl vandret** og **Spejl lodret** og derefter to kontakter: **Lås** bevarer materialeplaceringen, når en brush flyttes, og **UV** bevarer den, når en brush drejes (`U` slår den til og fra). Derefter følger det aktive værktøjs egne indstillinger: Brush-værktøjet viser formen og dens valgmuligheder, og Afrunding-værktøjet viser radius, flader og hjørner med **Anvend**. Yderst til højre er værktøjets navn, gitterstørrelsen (**Gitter 16** osv.; klik for at vælge en anden) og menuen **Gitterindstillinger** med Fastgør til gitter, Vis gitter, Forhåndsvis animationer, Vis udeladte flader og Keyframe-animation.

## Kortvisninger
Der er fire visninger: 3D-visningen og tre flade visninger, Top (XY), Front (XZ) og Side (YZ). Indstillinger → Visning → Layout vælger mellem Én rude, To ruder, Tre ruder og Fire ruder. Træk i skillelinjerne for at ændre rudernes størrelse. Et klik i en visning giver den tastaturet. `Space` skifter en rude, der viser én visning ad gangen, til dens næste visning, og `Ctrl+Space` maksimerer visningen med tastaturet og gendanner layoutet igen.

Højreklik uden at trække i en vilkårlig visning for at åbne en genvejsmenu for markeringen og det, der er under markøren. Den tilbyder gruppering, lag, sammenkædede grupper, oprettelse af entiteter, flytning af brushes ind i eller ud af en entitet, kopiering af et materialenavn og visning af det i materialebrowseren.

## Inspektøren
Inspektøren til højre har fire faner. Træk i dens venstre kant for at ændre bredden.

- **Kort** (`Ctrl+1`): Lag, Grupper, Plugins, Problemer, View filters og Tags.
- **Entitet** (`Ctrl+2`): entitetsbrowseren, entitetsdefinitionerne, egenskaberne for den markerede entitet og, for knapper og triggere, Animationsudløser.
- **Flade** (`Ctrl+3`): UV, Materialer, Farve, Fej, Afrunding, Patch samt Indfældning og affasning. Hver sektion kan foldes sammen.
- **Assets**: biblioteket med genanvendelige brush-modeller.

`Ctrl+5` skjuler eller viser inspektøren, og `Ctrl+Alt+T` skjuler eller viser værktøjslinjen, hvilket giver visningerne mere plads.

## Statuslinjen og outputruden
Statuslinjen langs bunden viser editorens seneste meddelelse. Fejl vises med rødt. Klik på linjen, eller tryk på `Ctrl+4`, for at åbne outputruden med den fulde historik over meddelelser. De fleste kommandoer rapporterer her, hvad de gjorde, for eksempel hvor mange brushes der blev klippet.

## Dialoger og udseende
Dialoger som Indstillinger, Eksportér og Render billede åbner i deres egne vinduer. Indstillinger → Visning → Farveskema vælger mellem Følg skrivebordet, Lys og Mørk. **Panelernes uigennemsigtighed (%)** på siden Oversigt gør panelerne gennemsigtige over visningerne. Vinduet tegner sine egne knapper i dit systems stil: native på macOS, Windows 11-stil på Windows og GNOME-stil på Linux.

# Navigation og tastatur
## Bevægelse i 3D-visningen
Hold højre museknap nede, og træk for at se dig omkring. Træk med midterste knap for at panorere. Hjulet flytter frem og tilbage. `W`, `A`, `S` og `D` flyver frem, til venstre, tilbage og til højre, `Q` stiger og `X` daler; hold `Shift` nede for større skridt. Med **Alt + træk med midterste knap flytter kameraet** slået til under Indstillinger → Mus flytter Alt med midterste knap langs synsretningen. Hastigheder, invertering og synsfeltet findes under Indstillinger → Mus.

## Bevægelse i 2D-visninger
Træk med midterste knap for at panorere, og brug hjulet til at zoome omkring markøren. Når du panorerer og zoomer i én flad visning, holdes de andre på linje; **Sammenkæd 2D-visninger** under Indstillinger → Mus styrer dette. `Ctrl+Alt+Z` nulstiller zoomen, og `Ctrl+Shift+U` sætter de flade visninger tilbage til origo.

`Ctrl+U` indrammer markeringen i alle visninger. Visning → Gå til (`Ctrl+Alt+M`) flytter kameraet til indtastede koordinater, markerer det, der står på en linje i kortfilen, eller flytter markeringen med en indtastet afstand.

## Gitter og fastgørelse
`+` og `-` ændrer gitterstørrelsen, og taltasterne `1` til `9` vælger en størrelse direkte, fra 1 til 256 enheder. `0` eller `#` viser eller skjuler gitteret, og `Alt+0` slår fastgørelse til eller fra. `S` fastgør de markerede brushes' hjørnepunkter til det aktuelle gitter, `Ctrl+Shift+V` fastgør dem til hele enheder og `Ctrl+Alt+Shift+V` til gitteret.

## Almindelige genveje
- `Ctrl+S`: gem kortet. `Ctrl+Shift+S`: gem som.
- `Ctrl+Z`: fortryd. `Ctrl+Shift+Z` eller `Ctrl+Y`: annuller fortryd.
- `Ctrl+C`, `Ctrl+X`, `Ctrl+V`: kopiér, klip ud og indsæt. Indsæt placerer objekterne ved markøren; `Ctrl+Alt+V` bevarer deres oprindelige koordinater. Med flader markeret tager kopiering deres materiale og UV-indstillinger.
- `Ctrl+D`: duplikér. `Ctrl+Shift+D`: sammenkædet dublet af en gruppe.
- `Delete` eller `Backspace`: slet markeringen.
- `Esc`: annuller det aktuelle træk, gå et skridt tilbage i det aktive værktøj, vend tilbage til Markér og ryd til sidst markeringen. `Shift+Esc` vender direkte tilbage til Markér.
- `Ctrl+R`: gentag de seneste gentagelige kommandoer. `Ctrl+Shift+R`: glem dem.
- `Ctrl+Alt+0` eller `Ctrl+Alt+Num0`: placér renderkameraet ved 3D-visningen.
- `F5`: genindlæs materialerne. `F6`: genindlæs entitetsdefinitionerne.

## Genveje til markering
- `Ctrl+A`: markér alt, eller alle håndtag, mens et værktøj til hjørnepunkter, kanter eller flader er aktivt. `Ctrl+Shift+A`: markér intet. `Ctrl+Alt+A`: invertér markeringen.
- `Ctrl+B`: markér de andre objekter i samme entitet eller gruppe.
- `Ctrl+T`: markér alt, som de markerede brushes rører. `Ctrl+E`: markér alt inden i dem. `Ctrl+Shift+E`: markér alt i deres søjle langs visningens akse. Disse bruger de markerende brushes op.
- `Ctrl+I`: isolér markeringen. `Ctrl+Alt+I`: skjul den. `Ctrl+Shift+I`: vis alt igen.
- `Ctrl+G`: gruppér. `Ctrl+Shift+G`: opløs gruppe.

## Genveje til transformation
- Piletasterne og `Page Up` / `Page Down` flytter markeringen ét gittertrin i den retning, de peger i den aktive visning. Med `Ctrl` duplikerer de under flytningen, og med `Alt` roterer de.
- `Alt+X`, `Alt+Y`, `Alt+Z`: rotér en kvart omgang om den akse; tilføj `Shift` for den modsatte retning.
- `Ctrl+F`: spejl på tværs af visningen; `Ctrl+Alt+F`: spejl op og ned. `Ctrl+Shift+X` og `Ctrl+Shift+Y` spejler langs X eller Y.
- `Insert`: placér en punktentitet i markeringens centrum.
- `Tab`: med en brush markeret skiftes Markér → Hjørnepunkt → Kant → Flade → Markér.

## Blender-tastaturgenvejene
Indstillinger → Tastatur → Tastaturgenveje tilbyder Heron Brush (standarden, som bruges i hele denne manual) og Blender. Med Blender: `G` markerer og flytter, `R` roterer, `S` skalerer, `E` fejer, `1`, `2` og `3` vælger værktøjerne til hjørnepunkter, kanter og flader, `K` klipper, `B` markerer, `A` markerer alt og `Alt+A` fjerner markeringen, `X` sletter, `Shift+D` duplikerer, `H` skjuler og `Alt+H` viser alt, og `Ctrl+Shift+Z` annullerer fortryd. Alt andet er ens i begge.

# Brushes og modelleringsværktøjer
Brushes er konvekse legemer defineret af planer. Byg komplekse objekter af flere simple brushes i stedet for at forsøge at gøre én brush konkav. Hold antallet af brushes lavt, hvor en simpel form er nok.

## Tegning af brushes
Vælg Brush-værktøjet (`B`), og træk i en visning. Værktøjslinjen viser **Form, der skal tegnes**: Kasse, Trappe, Buegang, Cylinder, Kegle, Sfæroide (UV), Sfæroide (ikosaeder) og Indsæt 3D-tekst. `Alt+1` til `Alt+7` vælger en form direkte. Afhængigt af formen viser linjen **Udhul** og **Tykkelse:**, **Trinhøjde:** og **Retning:** for trapper, **Akse:**, **Sider:** og cirkeltilstanden for runde former, **Ringe:** for kugler og **Nøjagtighed:** for ikosaederet. Indstil valgmulighederne, før du trækker. Kontrollér resultatet i mere end én visning.

## Markering
Klik på en brush for at markere den, og `Ctrl`-klik for at føje til markeringen. Et træk fra et tomt område tegner en markeringsramme. I 3D-visningen markerer `Shift`-klik en enkelt flade, og `Ctrl+Shift`-klik tilføjer flere flader. Når du trækker i en markeret brush, flyttes den; når du trækker i en umarkeret, markeres og flyttes kun den.

I 3D-visningen kan du `Shift`-trække i en flade på en markeret brush for at ekstrudere den. `Alt`-klik på en anden flade for at kopiere den markerede flades materiale og justering over på den; `Alt+Ctrl` kopierer kun materialet, og `Alt+Shift` kopierer rotationen.

## Formningsværktøjer
- **Markér** (`Esc`): markér og flyt.
- **Hjørnepunkt** (`V`), **Kant** (`E`), **Flade** (`F`): træk i hjørner, kanter eller flader. `Shift`-træk fra midten af en kant eller flade tilføjer et hjørne der. `Delete` fjerner de valgte hjørner.
- **Rotér** (`R`), **Skalér** (`T`), **Forskyd** (`G`): transformér markeringen med håndtag.
- **Klip** (`C`): klik på to eller tre punkter for at definere et snitplan. `Enter` skærer, `Ctrl+Enter` vælger, hvilken side der beholdes, og `Backspace` eller `Esc` fjerner det sidste punkt.
- **Saml** (`N`): placér punkter rundt om et volumen, og tryk på `Enter` for at lave en konveks brush af dem. `Backspace` fjerner det sidste punkt.
- **Fej** (`Y`): se afsnittet om fejning nedenfor.
- **Kontrolpunkt** (`P`): omform en patchs kontrolgitter.
- **Afrunding** (`O`): klik på en kant for at runde den af, eller vælg kanter eller hjørnepunkter med deres værktøjer, og tryk på `Enter`. Flere flader giver et blødere resultat og mere geometri.
- **Mal** (`K`): se Materialer og UV'er.

## Kombinering af brushes
- `Ctrl+J`: flet de markerede brushes til deres konvekse hylster.
- `Ctrl+K`: træk de markerede brushes fra de brushes, de rører.
- `Ctrl+Shift+K`: udhul markeringen med vægge, der er ét gittertrin tykke.
- `Ctrl+L`: dan fællesmængden af markeringen.

## Fejning, afrunding, patch og affasning
**Fej** bygger geometri langs en sti ud fra markerede flader. Markér fladerne (i Fej-værktøjet med `Alt`-klik), indstil derefter **Strækning**, **Drejning** (akse og vinkel), **Skalér**, **Trin** og **Sti** (justering Heltal eller Fri; Bue, Lige eller S-kurve) i sektionen Fej under fanen Flade, og tryk på **Fej**. Linjen under kontrollerne fortæller, hvor mange brushes der bliver lavet.

Sektionen **Afrunding** runder de valgte kanter af med **Radius**, **Flader** og **Hjørner** (Geret eller Afrundet); én flade giver en affasning.

**Patch** viser den markerede patchs **Kolonner** og **Rækker** med knapper til at tilføje eller fjerne et par. Geometri → Konvertér flader til patches (`Ctrl+P`) laver patches af markerede flader.

**Indfældning og affasning** virker på markerede flader: **Indfældning** trækker fladen ind fra dens kanter, og **Dybde** skubber den ud (eller ind, når værdien er negativ). **Affas flader** anvender begge; den tredje knap tilføjer et panel eller skærer en fordybning, afhængigt af fortegnet på Dybde. **Sammenføj hjørne** åbner en dialog, der sammenføjer to brushes i et geret eller afrundet hjørne med en forhåndsvisning.

## 3D-tekst
Geometri → Indsæt 3D-tekst (også det sidste punkt i Brush-værktøjets liste over former) bygger brushes af tekst. Den bruger Heron Block, en indbygget polygonskrifttype med afskårne hjørner, proportionale bredder og tjekkiske accenter; små bogstaver bruger versalformer. Skriv **Tekst**, indstil **Højde** (versalhøjden; accenter rager op over den) og **Dybde**, følg den levende forhåndsvisning, og tryk på **Indsæt**.

Hvis en handling ikke kan lave gyldig geometri, så justér dens mål, eller fortryd og forenkl formen.

## Arrays
Geometri → Array kopierer markeringen og viser kopierne i kortet, mens du ændrer værdierne. Antallene medregner originalen, og et array laver højst 1024 kopier.

- **Række** flytter hver kopi i forhold til den forrige med **Trin X**, **Trin Y** og **Trin Z** og drejer den **Drejning pr. kopi (grader)** om den valgte akse.
- **Gitter** fylder **Antal X**, **Antal Y** og **Antal Z** elementer med afstanden **Trin X**, **Trin Y** og **Trin Z**.
- **Ring** fordeler elementerne over **Vinkel (grader)** rundt om et centrum **Radius (enheder)** fra markeringen. Med **Vend mod centrum** drejer hver kopi med ringen.
- **Spredning** placerer kopierne tilfældige steder inden for radius, på tværs af den valgte akse. Samme **Seed** giver samme placering, og **Tilfældig drejning** drejer hver kopi.

Når markeringen er én hel gruppe, gør **Sammenkædede kopier** hver kopi til en sammenkædet kopi, så en ændring af én når dem alle. **Anvend** beholder kopierne og markerer dem sammen med originalen; **Annuller** fjerner dem. Hele arrayet fortrydes i ét trin.

## Modellér terræn
Terræn består af mange små brushes, hvis overflade modelleringspenslen skubber ud og trykker ind. Markér en eller flere brushes, og vælg Geometri → Del i søjler. Det skærer dem langs gitterlinjerne i søjler, ét gittertrin brede, og lægger søjlerne i en gruppe ved navn Terrain. Snittene følger verdens gitter, så brushes, der ligger side om side, skæres de samme steder og deler hjørner. Hver søjle står langs brushens tynde side: lodret i et gulv eller et loft, vandret i en væg. En hule laves på samme måde: gør en kasse hul (Hollow selection) med en vægtykkelse på ét gittertrin, markér væggene, og del dem.

Vinduet **Modellér**, som knappen for enden af modelleringslinjen åbner, vælger søjlernes form og kan også dele markeringen. **Trekant** laver to pr. gittercelle, og oversiden af en trekant forbliver plan, uanset hvordan den modelleres. **Kvadrat** laver én pr. celle, altså halvt så mange brushes; dens overside knækker langs en diagonal. **Sekskant** dækker brushen med sekskanter, to gittertrin brede, hver delt i seks trekanter om midten, så hver overside forbliver plan.

Vælg værktøjet **Modellér** (`J`), og træk hen over overfladen i 3D-visningen. Den orange ring viser penslen. Uden markering ændrer penslen alle brushes under den; med en markering kun de markerede. Linjen øverst i vinduet vælger, hvad et strøg gør:

- **Hæv** og **Sænk** flytter overfladen ud eller ind, mest i midten af ringen og slet ikke ved kanten. Hold `Shift` nede for at bytte dem.
- **Glat** udglatter buler.
- **Udjævn** jævner overfladen ud til det sted, hvor strøget begyndte.

I samme vindue vælger **Retning** hvilken vej:

- **Op** flytter overfladen lodret op og ned, inden for en cirkel på jorden. Den er til terræn.
- **Kamera** flytter den mod visningen og væk fra den, inden for en kugle omkring markøren. Se på et gulv, en væg eller et loft, og penslen former det.
- **Normal** flytter den ud fra den flade, der er under markøren, hvor strøget begynder.

De to tal er **Radius (enheder)** og **Styrke (%)**. Kun hjørnerne på den side, strøget flytter, bevæger sig; bagsiden bliver, hvor den er, og en søjle forbliver mindst én enhed tyk. Hvor brushes deler et hjørne, flytter de det sammen, så overfladen forbliver lukket. Et strøg fortrydes i ét trin. I 2D-visningerne markerer værktøjet som markeringsværktøjet.

# Materialer og UV'er
## Indlæsning og anvendelse af materialer
Åbn sektionen **Materialer** under fanen Flade. **Indlæs mappe** tilføjer en mappe med billeder, og **Indlæs .wad** tilføjer en teksturpakke; begge vises under **Samlinger**, hvor krydset fjerner en. Skriv i **Filtrer materialer** for at indsnævre listen. Klik på et materiale for at gøre det aktuelt, hvorefter **Anvend på markering** lægger det på alle markerede flader. **Markér** tilbyder Markér brushes med det og Markér flader med det. **Erstat** udskifter ét materiale med et andet i hele kortet eller kun i markeringen.

Værktøjet **Mal** (`K`) lægger det aktuelle materiale på fladen under markøren: et klik maler én flade, et dobbeltklik maler hele brushen, og `Shift`-klik opsamler fladens materiale som det aktuelle, ligesom en pipette.

## UV-editoren
Sektionen **UV** under fanen Flade viser den markerede flades materiale og omrids. Musestyring på lærredet:
- Hjulet zoomer omkring markøren. Træk med højre eller midterste knap for at panorere. Dobbeltklik med højre knap for at indramme fladen igen; når du markerer en anden flade, indrammes den automatisk.
- Træk med venstre knap flytter materialet, mens fladens omrids bliver på plads. Ringen roterer placeringen, hjørnehåndtagene skalerer den, og midterhåndtaget angiver udgangspunktet for disse ændringer.
- `Alt`-træk med venstre knap på en flisekant forskyder, ligesom `Shift`-træk med midterste knap.
- Hvert træk er ét fortrydelsestrin, og `Esc` annullerer et igangværende træk.

Under lærredet er **Forskydning**, **Skalér**, **Vinkel** og, når spillet bruger dem, **Indholdsflag**, **Overfladeflag** og **Overfladeværdi**. Knapperne er **Nulstil**, **Nulstil til verden**, **Tilpas**, **Spejl H**, **Spejl V** og de to drejninger på 90°. **Juster** drejer materialet langs en af fladens kanter (tryk igen for den næste kant, `Shift` for den forrige). De fire justeringsknapper skubber materialet op mod en kant. **Tilpas U** og **Tilpas V** skalerer, så et helt antal kopier dækker fladen; tryk igen for det næste antal, og hold `Ctrl` nede for at bevare et trim sheets inddeling.

## UV-taster
Med flader markeret flytter piletasterne materialet, `[` og `]` drejer det 15°, `,` og `.` halverer eller fordobler dets skala, `Ctrl+F` og `Ctrl+Alt+F` spejler det, `Shift+R` nulstiller det, `Alt+Shift+R` nulstiller det til verdensakserne, og `Ctrl+Shift+F` tilpasser det til hver flade.

**Lås** i værktøjslinjen bevarer placeringen, mens brushes flyttes, og **UV** (`U`) bevarer den, mens brushes drejes.

## Værktøjsmaterialer
- skip: fjerner den markerede flade fra motoreksporter.
- clip: skaber kollision uden synlig geometri.
- origin: markerer omdrejningspunktet for en brush-entitet.

Navnene kan ændres i Eksportér-vinduets kort **Værktøjsmaterialer**. Brug skip til flader, som du ved er skjulte.

## Udeladelse af flader fra eksport
Markér en flade, og slå **Udelad flade fra eksport** til i sektionen UV. Orange skravering markerer den i UV-forhåndsvisningen. Dens materiale og maling forbliver gemt, og brushen forbliver lukket og redigerbar. Eksporterede render-meshes udelader fladen, mens kollisionen bevares. Udelader du én firkant af en terning, giver det to render-trekanter. Det gør ikke flader dobbeltsidede og fletter ikke meshes.

**Udelad tildækkede flader** markerer flader, der er fuldstændig dækket af statiske brushes, herunder identiske flader, der rører hinanden, og indesluttede brushes. Det virker på de markerede brushes eller på al synlig, ulåst verdensgeometri, når intet er markeret. Brush-entiteter, gennemsigtige materialer og brushes, der allerede har udeladte flader, springes over. Delvist tildækkede flader deles ikke. Én fortrydelse gendanner det hele. Visning → Vis udeladte flader skjuler eller viser sådanne flader i editorens visninger; udpegning bruger stadig hele brushen.

# Farve og gradienter
Sektionen **Farve** under fanen Flade maler flader med ensfarvede farver eller gradienter oven på deres materialer. Den er nyttig til at skitsere en bane og til stiliseret grafik. Malede flader beholder deres egentlige farver i visningerne; markeringen vises med sit omrids.

## Palet og blandingsfarve
Klik på en af de tolv farveprøver i paletten for at male de markerede flader eller hele markerede brushes med den. **Uigennemsigtighed** angiver i procent, hvor dækkende den næste maling er. **Bland** har en farveknap, der åbner farvevælgeren; træk i vælgeren, og markeringen følger med live, eller tryk på **Mal** for at anvende blandingsfarven som ét trin. **Tilbage til materiale** fjerner malingen, så fladen igen viser sit materiale. Linjen øverst i sektionen fortæller, hvad der er markeret, og hvad det har på.

## Farvevælgeren
Alle farveknapper i editoren åbner den samme vælger. Træk i kvadratet for at vælge mætning og lysstyrke, og i strimlen ved siden af for at vælge farvetone. Nedenunder er værdierne **R**, **G** og **B** og en skyder til **Uigennemsigtighed**, hvor farven har en. Hex-feltet tager `#RRGGBB` eller `#RRGGBBAA`; det bliver rødt, så længe teksten ikke er en gyldig farve. **Kopiér** lægger hex-koden i udklipsholderen. Klik uden for vælgeren for at lukke den.

## Gradienter
**Gradient** vælger Lineær gradient eller Radial gradient, og knappen ved siden af vender farverne om. Hvert stop har en farveknap og **Position (%)**, og stop mellem enderne har en fjern-knap. **Tilføj farvestop** tilføjer et stop i det største mellemrum; en gradient kan have op til otte stop. **Gradientvinkel** angiver retningen for en lineær gradient med knapperne **+45°** og **−45°**.

Under forhåndsvisningen er en skinne med en markør for hvert stop. Klik på skinnen for at tilføje et stop, klik på en markør for at åbne dens vælger, træk i en markør for at flytte den, og højreklik på en markør for at fjerne den. De tre farveprøver under skinnen er forudindstillinger: Varm gradient, Kølig gradient og Gråtonegradient. Ændringer af en gradient anvendes live på markeringen.

En gradients retning har betydning: læg den langs en tagflade eller et buet objekt, ikke på tværs af flader, der ikke hører sammen. Kontrollér modellen fra begge sider efter malingen.

## Overflademateriale
**Overflademateriale** indstiller **Ruhed** (0 er blank, 100 er mat), **Metallisk** (0 for træ, sten eller plast, 100 for metal) og **Emission** (0 til 8; overfladen gløder i sin egen farve, også gradienter, men oplyser ikke objekter i nærheden). Ændringer anvendes live på de markerede flader eller brushes. **Brug standardoverflade** fjerner tilsidesættelsen. Alle ændringer kan fortrydes.

Overfladeeffekter vises i den oplyste 3D-visning og i kamerarenderinger. Flad visning og trådmodel udelader bevidst højlysene. Bloom kan tilføje en glød omkring selvlysende overflader. Godot og glTF/GLB modtager materialeværdierne, og Unity-importøren omsætter ruhed til glathed. Se `docs/surface-materials.md` for begrænsningerne ved hvert mål.

# Assets, lag og grupper
## Asset-biblioteket
Åbn fanen **Assets**. Tilføj en mappe med `.map`-modeller med **Tilføj mappe**; det indbyggede bibliotek er der altid. Skriv i **Filtrer assets** for at søge. Klik på en model for at markere den, og dobbeltklik derefter på den, tryk på `Enter`, eller klik på **Indsæt** for at tilføje den, hvor markøren sidst var. Du kan også trække en model ind i en visning. Asset-brushes indsættes som én gruppe, så modellen flyttes samlet.

**Gem markering som asset** skriver de markerede brushes til biblioteket som en `.map`. Biblioteksassets er almindelige redigerbare brushes, ikke importerede meshes.

## Lag
Listen **Lag** under fanen Kort organiserer store områder. Skriv et navn, og klik på **Tilføj** for at lave et lag, eller tryk på `Enter` for at omdøbe det aktuelle. Hvert lag kan skjules, låses eller udelades fra eksporter. **Flyt hertil** flytter markeringen ind i det aktuelle lag. Genvejsmenuen i visningerne tilbyder også Flyt til lag, Gør til aktivt lag, Skjul lag og Isolér lag.

## Grupper og sammenkædede grupper
`Ctrl+G` grupperer markeringen, og `Ctrl+Shift+G` opløser gruppen. Sektionen **Grupper** under fanen Kort viser grupper og tilbyder **Åbn**, **Omdøb**, **Flet**, **Opløs gruppe**, **Tag ud** og **Udtræk**. En sammenkædet gruppe er et sæt kopier, der deler redigeringer: `Ctrl+Shift+D` laver en sammenkædet dublet, Geometri → Synkronisér sammenkædede grupper opdaterer kopierne, og **Fjern sammenkædning** gør en kopi uafhængig. I fanen Entitet kan en egenskab beholdes som en kopis egen, når sammenkædede grupper synkroniseres.

## En fælles skala
Standardskalaen for eksport er 32 kortenheder pr. meter. Den indbyggede markør for spillerstart er 64 enheder høj, en reference på 2 meter. Sammenlign døråbninger, køretøjer, møbler og planter med den. Ændr kun eksportskalaen med vilje, og hold relaterede assets ensartede.

# Entiteter og gameplay
Almindelige entitetsdefinitioner er indbygget og altid tilgængelige: lys, spillerstarter, døre, knapper, platforme, triggere, statiske rekvisitter, lyde og waypoints. Indlæser du en projekt-FGD med **Entitetsdefinitioner** i fanen Entitet, erstattes klasser med samme navn, mens de øvrige indbyggede bevares.

## Placering af entiteter
I browseren **Entiteter** under fanen Entitet skriver du i **Filtrer klasser** og vælger en klasse. For en punktklasse som et lys klikker du på **Placér** eller dobbeltklikker på klassen; `Insert` placerer en i markeringens centrum. For en brush-klasse som en dør eller en trigger markerer du brushes og klikker på **Opret**. Klasser kan også trækkes ind i en visning.

## Egenskaber
Egenskabslisten redigerer den markerede entitet. Entiteter af samme klasse redigeres samlet. **Tilføj egenskab** tilføjer en nøgle, som definitionen måske ikke kender; **Standarder** kan sætte eksisterende egenskaber tilbage til deres standardværdier eller tilføje de manglende. Egenskaber, som klassen definerer, men som entiteten ikke har, vises nedtonet; når du skriver en værdi, tilføjes de. Brug targetname og target til at forbinde entiteter.

## Hvad en eksport kan
Godots direktiver opretter native knuder, kollisionslegemer og triggerområder. Unity og Unreal understøtter native almindelige lys, og Unreal opretter også PlayerStart-aktører. Andre data bevares, så vidt målets importør tillader det.

En dørentitet alene tilføjer ingen spilopførsel. Bevægelse, reaktioner på triggere, spillerstyring og projektscripts kræver stadig arbejde i motoren. Defold eksporterer entitetsegenskaber og kollisionsobjekter; Flax placerer modeller og generiske entiteter. Kontrollér resultatet i din motor, før du bygger gameplay oven på det.

# Animation
Heron Brush kan forhåndsvise simpel bevægelse og redigere keyframe-animationer af brush-entiteter og derefter eksportere dem til flere motorer.

## Forhåndsvis animationer
Slå **Forhåndsvis animationer** til i menuen Visning, menuen Animation, menuen Gitterindstillinger eller sektionen Visning under fanen Kort. Den afspiller func_rotate-rotation og func_shuttle-frem-og-tilbage-bevægelse, når start_on er slået til, ud fra entitetens egenskaber for akse, hastighed, retning, strækning og ventetid, og den afspiller keyframe-klip, der er sat til Afspil ved start. Ventilator- og vindmølle-assets indeholder allerede roterende dele.

Når forhåndsvisningen slås fra, gendannes den oprindelige stilling. Et klik i en visning stopper afspilningen, før der markeres eller redigeres; kameranavigation virker fortsat. Forhåndsvisningen ændrer ikke kortet, fortrydelseshistorikken eller eksporter, og den kører hverken fysik eller spillogik.

## Keyframe-editoren
Åbn den med Animation → Keyframe-animation eller fra menuen Gitterindstillinger. Den dokkes under visningerne eller i 3D-vinduet, mens det er separat. Vinduesikonet i dens overskrift flytter den ud i sit eget vindue; den beholder billedet og fortrydelseshistorikken. **Rediger animation** skifter visningerne til den animerede stilling, og **Rediger kort** går tilbage til hvilegeometrien. I animationstilstand viser hver visning **Rediger animation** og billednummeret nederst.

Markér en brush-entitet eller en brush eller brush-gruppe for at animere den. En flad brush-gruppe deler én animation og ét omdrejningspunkt; grupper, der indeholder flere entiteter, kræver separate animationer.

## Klip
Vælgeren **Animation** skifter mellem navngivne klip. Skriv et navn i **Animationsnavn**, og klik på **Gem som** for at kopiere det aktuelle klip under det navn; **Slet** fjerner et klip. **Flyt** opretter et `translate`-klip uden rotation. **Døranimationer** opretter bløde `open`- og `close`-klip med en drejning på 90 grader om det aktuelle omdrejningspunkt; flyt først omdrejningspunktet til hængslet. **Afspil ved start** får klippet til at spille, når banen starter; dørforudindstillingen slår det fra.

## Transport og afspilning
Transportknapperne er **Start**, **Forrige nøgle**, **Afspilning**, **Næste nøgle** og **Slut**. **Billede** viser og indstiller det aktuelle billede, og **Varighed (s)** angiver klippets længde. Tilstanden er **Én gang**, **Løkke** eller **Frem og tilbage**.

## Tidslinjen
Tidslinjen har en lineal med afspilningshovedet og et spor for **Position** (X, Y og Z sat som nøgle sammen) og for hver rotationsvinkel, RX, RY og RZ. Klik eller træk i linealen for at flytte afspilningshovedet og forhåndsvise den stilling i alle visninger. Træk med midterste knap for at panorere tidslinjen. Træk i nøgler for at flytte dem i tid; `Shift` føjer til markeringen; dobbeltklik på et spor for at indsætte en nøgle; `Ctrl`+hjul zoomer. **Kurver** viser værdikurverne i stedet for sporene.

Knapperne over den er **Tilføj nøgle**, **Fjern**, **Kopiér**, **Indsæt** og **Duplikér**. **Fastgør til billeder** holder nøgler på hele billeder, og **Tilpas** viser hele klippet. **Omtime nøgler** åbner **Tidsskala**, **Forskydning (billeder)**, **Skalér fra afspilningshovedet**, **Forlæng varighed** og **Omtime markering**.

Nøgleinspektøren ved siden af tidslinjen redigerer den markerede nøgle: **Billede**, **Værdi** (eller de tre koordinater for en Position-nøgle), interpolationen **Lineær**, **Hold** eller **Bézier**, **Indgående stigning** og **Udgående stigning** samt **Udglat tangenter**.

## Taster i editoren
Mens tidslinjen har tastaturet: `Space` afspiller eller sætter på pause, venstre og højre piletast går ét billede, `Ctrl+A` markerer alle nøgler, `Ctrl+C`, `Ctrl+X` og `Ctrl+V` kopierer, klipper ud og indsætter nøgler, `Ctrl+D` duplikerer dem, `Delete` fjerner dem, `Ctrl+Z` fortryder, og `Esc` annullerer et træk. Uden for tidslinjen forlader `Esc` animationstilstanden. Fortryd lader zoom, markering og afspilningshoved blive, hvor de var.

## Stillinger i visningerne
Rækken under tidslinjen har **Løgskind** med et antal og **Afstand**, som viser nærliggende stillinger som spøgelser; koordinaterne for **Omdrejningspunkt**; **Automatisk nøgle**; og stillingsværktøjerne **Flyt**, **Rotér** og **Omdrejningspunkt**.

- **Tilføj nøgle** slår Automatisk nøgle til og sætter afspilningen på pause. Med Automatisk nøgle slået til og **Flyt** valgt trækker du i en farvet akse på den markerede entitets håndtag i 3D-visningen for at sætte nøgle på dens position i det aktuelle billede.
- I en 2D-visning trækker du i den markerede brush eller et medlem af dens gruppe for at sætte nøgle på flytning langs de to synlige akser, med fastgørelse til gitteret. Automatisk nøgle behøver ikke at være slået til for dette.
- Med **Rotér** trækker du brushen i en 2D-visning rundt om dens orange markør for omdrejningspunktet. Top sætter nøgle på RZ, Front på RY og Side på RX, fastgjort til gittervinkler.
- Med **Omdrejningspunkt** trækker du det gule rotationscentrum i en vilkårlig 2D-visning. Det fastgøres til gitteret og beholder sin dybde. Nøgler og hvilegeometri forbliver uændrede.

Den cyanfarvede sti i 2D og 3D viser omdrejningspunktets bane. Klik på et punkt på stien for at markere dets tid og nøgler, eller træk i det for at ændre positionen; med Automatisk nøgle slået til kan du også trække i stipunkter i 3D-visningen. Etiketter viser billednumre, og stiplede segmenter markerer spring. `Esc` annullerer ethvert af disse træk, og én fortrydelse omgør et helt træk.

Nye animationer starter i hvilestilling med nøgler på tidspunkt nul. Skab bevægelse ved at ændre stillingen i et andet billede. For en skarp L-formet sti sætter du nøgle på starten, hjørnet og slutningen og bruger Lineær med Frem og tilbage.

## Forbindelse af knapper til animationer
Markér en func_button, trigger_multiple eller trigger_once. Fanen Entitet viser **Animationsudløser** over egenskaberne. Vælg et mål på listen, eller klik på **Udpeg mål** og derefter på et animeret objekt i en vilkårlig visning; `Esc` annullerer udpegningen. Manglende mål eller klip rapporteres under kontrollerne. Entitetsforbindelser viser pile til deres mål, når visning af forbindelser er slået til.

**Handling** vælger, hvad aktivering gør: **Afspil** starter et navngivet klip eller den markerede animation; **Slå til/fra** skifter mellem `open` og `close`, når de findes, og ellers genstarter den den markerede animation; **Åbn** og **Luk** starter disse klip; **Pause** fastholder stillingen, **Genoptag** fortsætter, og **Stop** gendanner den oprindelige stilling. **Kun én gang** tilsidesætter klassens standard. **Gentagelsesforsinkelse (s)** angiver den korteste tid mellem aktiveringer; den forsinker ikke den første.

**Afprøv udløser** kører handlingen i forhåndsvisningen uden at redigere kortet eller tilføje fortrydelsestrin. **Nulstil forhåndsvisning** gendanner de oprindelige stillinger og rydder tilstanden for én gang og nedkøling. Dette simulerer hverken spillerkollisioner eller spilscripts.

## Animation i eksporter
Godot eksporterer native AnimationPlayer- og AnimationLibrary-ressourcer, inklusive kurver og bevægelig kollision. En CharacterBody3D, der træder ind i en knaps område, aktiverer dens mål. Ændringer af disse klip i Godot kommer ikke tilbage til kortet og erstattes ved næste eksport.

Raylib-eksporten tilføjer `<map>_animation.h` med en afspiller i C og `<map>_preview.c` med en forhåndsvisning med frit kamera; følg `<map>_raylib.md` for at bygge den. I forhåndsvisningen sætter `SPACE` på pause, `R` genstarter standardklippene, og `N` skifter mellem navngivne klip. Defold indlejrer afspilning i entitetsscriptet med dokumenterede `heron_*_animation`-meddelelser. Unitys Build Level tilføjer `HeronBrushAnimation` og et kinematisk legeme. Bevy-eksporten indeholder `heron_animation.rs` og en vejledning. Godot, Unity, Defold, Bevy og raylib bevarer triggerforbindelserne, én gang-flaget og gentagelsesforsinkelsen. Dit spil skal stadig registrere spilleren og forbinde sine egne knapper til hver runtimes API.

## Eksemplet Motion lab
Åbn `motion_lab` fra velkomstskærmens Eksempler. Det har en platform med L-formet sti, en elevator, en roterende platform og en hængslet dør med knapper. Åbn `godot/project.godot` ved siden af i Godot, og tryk på `F5`. WASD og musen flytter spilleren, Mellemrum hopper, 1 til 4 besøger stationerne, og R genstarter den aktuelle station. Træd på en farvet plade for at aktivere dens mål. Den medfølgende README forklarer, hvordan man eksporterer igen.

# Eksport til en motor
Fil → Eksportér åbner vinduet Eksportér. Motorerne står i en liste i venstre side med deres logoer: Godot, Defold, three.js, Bevy, raylib, Unity, Unreal Engine, Flax Engine, Cocos Creator, Wavefront .obj, glTF .gltf, GLB .glb og sidst `.map`. Siden til højre fortæller, hvad målet får. **Skriv** skriver det, **Kontrollér** bygger banen uden at skrive den og viser problemerne, og **Luk** lukker vinduet.

Eksport er adskilt fra gemning: `.map` forbliver din redigerbare kilde. Hver kontrol gemmer sin motors indstillinger, så snart den ændres, så hver motor beholder sine egne svar, og vinduet åbner, som du forlod det.

## Fælles valgmuligheder
- **Skalér**: **Enheder pr. meter**, 32 som standard.
- **Hvad der skal skrives**: **Kollisionsgeometri**.
- **Optimeringer**: **Gruppér statiske meshes i celler på 1024 enheder**, **Fjern flader, der er begravet i geometrien**, **Saml flader, der deler et plan** og **Saml kollisionsformer**.
- **Projektstier**: hvor banen og dens ressourcer placeres i projektet.
- **Værktøjsmaterialer**: navnene på materialerne clip, skip og origin.
- **Mere**: motorspecifikke kontakter som **Navngiv knuder efter**, **Genopbyg lag og grupper som overordnede knuder**, **Vend trekanternes omløbsretning**, spejling af teksturkoordinater og et andet UV-sæt til bagt lys med **Lightmap-texel**.
- **Holder trit med kortet**: **Skriv igen, hver gang kortet gemmes**. Det bliver tilgængeligt, når motoren er blevet eksporteret én gang, fordi den eksport fortæller, hvor der skal skrives.

Statiske celler fordeler verdens-brushes efter deres centrum og kombinerer dem efter materiale inden for hver celle på 1024 enheder, så motorer kan frasortere mindre dele af kortet. Kollisionen bevares, og gameplay-entiteter forbliver adskilte. Det kan betyde flere draw calls end ét samlet verdens-mesh, så det er slået fra som standard. Selvstændige OBJ-, glTF- og GLB-eksporter bruger ikke celler.

## Noter om målene
- Godot: `res://`-stier til teksturer og materialer, fysiklaget for triggere og lightmap-UV2. Vælg en eksportmappe inde i Godot-projektet.
- Defold: assetmappen til ressourcer, modellens materiale og tekstur-sampleren, som skal findes i det valgte materiale.
- Unity: OBJ-meshes og en editorimportør; eksportér under projektets Assets-mappe.
- Unreal Engine: et genereret importscript og en indholdssti, normalt /Game/<level>.
- Flax Engine: en indholdssti til modellen i forhold til projektets Content-mappe.
- three.js, Bevy, raylib og Cocos Creator: scener eller kode til disse runtimes med en vejledning ved siden af eksporten.
- glTF / GLB: portabel geometri til kompatible værktøjer; GLB er den binære form. OBJ: geometri og materialer til modelleringsværktøjer.
- `.map`: **Skriv som** vælger Valve 220, Standard, Quake 2 eller Quake 2 (Valve). Vinduet fortæller, hvad den valgte dialekt ville miste.

Læs advarslerne efter skrivningen, for eksempel om manglende teksturer. **Hvad kontrollen fandt** viser dem i vinduet.

# Live-forbindelser: Godot, Unity, Defold og Flax
En live-forbindelse sender ikke-gemte ændringer fra Heron Brush til en motors editor, mens du arbejder. Den er envejs: ændringer foretaget i motoren kommer ikke tilbage. Der kan køre én live-forbindelse pr. kortvindue. Stop den, før du skifter motor, og genstart den for at anvende ændrede eksportindstillinger. Kontrollerne findes i vinduet Eksportér, på motorens side, i kortet **Live-forbindelse med**.

Indikatoren i titellinjen viser forbindelsens tilstand; klik på den for at stoppe forbindelsen. Når kortet lukkes eller udskiftes, stopper udgivelsen. Efter en mislykket import retter du problemet og starter forbindelsen igen.

## Godot
1. Eksportér én gang ind i et Godot-projekt.
2. Klik på **Installer Godot-tilføjelse**.
3. Slå Heron Brush Live til i Godots Project Settings → Plugins.
4. Klik på **Start live-forbindelse**, og tilknyt kortet fra Godot-dokken til en åben 3D-scene.

Tilføjelsen erstatter kun sin genererede gren og bevarer tilknytningens placering og andre knuder. Gem selv Godot-scenen, og hold dine egne scripts og knuder uden for den genererede gren.

## Unity
1. Eksportér én gang under Unity-projektets Assets-mappe.
2. Klik på **Installer Unity-tilføjelse**, og lad Unity kompilere den.
3. Åbn Window → Heron Brush → Live Link i Unity.
4. Start forbindelsen, og tilknyt kortet til den aktive Unity-scene.

Hold Unitys live-vindue åbent. Opdateringer holder pause i Play Mode og Prefab Mode. Kun kortobjektets underobjekt Generated erstattes. Rediger ikke de genererede filer.

## Defold
1. Eksportér én gang ind i dit Defold-projekt, og hold projektet åbent i Defold.
2. Klik på **Start live-forbindelse**. Der kræves ingen tilføjelse.
3. Åbn heron_brush_live/preview.collection i Defold, eller henvis til den fra din egen scene.

Forbindelsen skriver til en separat genereret mappe og lader den endelige eksport og game.project være i fred. **Genindlæs kørende spil** opdaterer også et spil, der er startet fra Defold; tilføjelse af objekter eller ændring af kollision kan kræve, at banen genstartes. Mislykkede forbindelser forsøges igen automatisk.

## Flax
Eksportér ind i projektets Content-mappe, og klik på **Installer Flax-tilføjelse**. Lad Flax kompilere den, start forbindelsen, og vælg derefter Tools → Heron Brush: attach live map i den åbne Flax-scene. Den genererede rod beholder sin placering, og andre objekter røres ikke. Gem selv scenen. Opdateringer holder pause i Play Mode. Tools → Heron Brush: stop live map sætter modtagelsen på pause.

# Fælles live-sessioner
Samarbejde → Live-session åbner et vindue, hvor du kan være vært for eller deltage hos en anden Heron Brush-bruger. Dette deler kortredigering mellem personer; det er adskilt fra en live-forbindelse til en motor.

## Vær vært og deltag
Indtast **Dit navn**. For at være vært vælger du **Vær vært for en session**, klikker på **Start session** og sender oplysningerne fra **Kopiér forbindelsesoplysninger** til dine gæster. For at deltage vælger du **Deltag i en session**, indsætter oplysningerne og klikker på **Deltag**. Når du deltager, erstattes det åbne kort med værtens kort, så gem dit arbejde først.

Standardudgaven virker på lokalnetværket, og **Port på lokalnetværk** angiver porten. Sessioner over internettet (**Forbind over internettet**) kræver en udgave med den valgfri netværksfunktion. Deltagerne ser hinandens kameramarkører og navne i 3D-visningen.

Når vinduet lukkes, fortsætter sessionen; indikatoren i titellinjen viser dens tilstand. **Stop session** eller **Forlad session** afslutter den. Når to personer redigerer det samme objekt, afgør værtens rækkefølge den endelige version, og fortryd er også en fælles redigering. Arbejd så vidt muligt i hver sit område, og gem kontrolpunkter.

# 3D-visning og en ekstra skærm
## Visningstilstande
Sektionen **Visning** under fanen Kort skifter, hvad visningerne tegner: Brushes, Punktentiteter, Entitetsmodeller, Klassenavne for entiteter, Gruppeomrids, Patches, Materialer, Vis udeladte flader, Forhåndsvis animationer, Trådmodel, Kanter og Omrids af brush-entiteter. Trådmodel tegner trekanternes kanter, inklusive diagonaler og kanter bag overflader, uden materialer. Det er en midlertidig fejlfindingsvisning og påvirker hverken eksporter eller renderinger.

Belysning, skygger, ambient occlusion, refleksioner, bloom og udglatning for 3D-visningen findes under Indstillinger → Renderer. Det er indstillinger for forhåndsvisningen; endelige billeder bruger renderkameraets egne indstillinger.

## Et separat 3D-vindue
Visning → 3D-visning i separat vindue flytter 3D-visningen ud i sit eget vindue, som du kan placere på en anden skærm. Det deler kamera, dokument og værktøjer, og de flade visninger overtager pladsen i hovedvinduet. Dets menu tilbyder Materialer, Massiv visning og Trådmodel; en undermenu Visning med Vis udeladte flader, Forhåndsvis animationer, Kanter, Gruppeomrids, Punktentiteter, Entitetsmodeller og Omrids af brush-entiteter; Fokusér på markering; Forfra, Side og Top, som drejer kameraet rundt om markeringen (eller et punkt foran kameraet); Placér kamera fra 3D-visning; Fuld skærm (`F11`); Render billede; og Tilbage til hovedvinduet. Når vinduet lukkes, dokkes visningen igen.

## Procedurel himmel
Den procedurelle himmel er slået til som standard i 3D-visningen og kan slås fra under Indstillinger → Renderer → Lys. Renderkameraer har deres egen kontakt **Procedurel himmel**, som gemmes med kortet. Farverne for himmel, horisont og jord kommer fra miljøindstillingerne, og solen følger scenens belysning. Himlen har ingen geometri og eksporteres ikke.

# Renderkameraer og PNG-billeder
Fil → Render billede åbner render-vinduet: kameraet og dets indstillinger til venstre og en levende forhåndsvisning til højre, der tegnes igen, når du ændrer dem. Kameraer gemmes i `.map` i et laget Render cameras, der kun findes i editoren og udelades fra motoreksporter.

## Placering af kameraet
Indram billedet i 3D-visningen, og tryk på `Ctrl+Alt+0` (eller `Ctrl+Alt+Num0`), eller klik på **Placér kamera fra 3D-visning**. Hvis intet kamera er markeret, opdateres det første gemte; hvis der ikke findes noget, oprettes et. Det kan fortrydes og bevarer indstillingerne for output og belysning. **Markér kamera i kortet**, **Indlæs kamera fra kort** og **Flyt 3D-visning til kamera** virker den anden vej. **Kameraramme i 3D-visning** viser outputtets afgrænsning, og **Styr kameraet fra 3D-visningen** lader almindelig navigation flytte det gemte kamera.

## Indstillinger
- **Kameraposition**: X, Y, Z, kurs og hældning.
- **Output og objektiv**: **Forudindstillinger** (Full HD, Kvadrat, Portræt), bredde, højde, samples, **Lodret synsfelt** og **Fjern klippeplan**.
- **Belysning**: Dagslys, Solnedgang eller Overskyet, eller indstil eksponering, solen, farverne for himmel, horisont og jord, **Procedurel himmel**, **Skygger**, **Ambient occlusion** og **Bloom**.
- **Dybdeskarphed**: **Slå dybdeskarphed til**, fokusafstand, skarphedsområde og sløringsradius. **Fokusér på markering** fokuserer på centrum af et markeret objekt foran kameraet.

**Gem kamera og indstillinger** gemmer formularen i kortet; gem kortet for at bevare det på disken.

## Rendering
**Render PNG** spørger, hvor der skal gemmes, og starter en progressiv rendering, der udfyldes sample for sample i forhåndsvisningen. Flere samples ser bedre ud, men tager længere tid. **Annuller rendering** stopper uden at skrive en fil. Rendereren bruger editorens rasterpipeline, ikke path tracing. PNG-filen har en uigennemsigtig baggrund, og animationer renderes ikke. Meget store billeder kan kortvarigt sætte editoren på pause.

# Indstillinger
Fil → Indstillinger (`Ctrl+,`) åbner indstillingerne. Ændringer træder i kraft med det samme og gemmes. Sidepanelet viser Oversigt, Spil, Visning, Farver, Mus, Tastatur og Renderer. Søgefeltet øverst finder en indstilling ud fra navn eller beskrivelse på alle sider. Hver ændret indstilling har sin egen knap til at gendanne standardværdien, og **Gendan standarder** nulstiller den aktuelle side.

## Siderne
- **Oversigt**: de almindelige indstillinger samlet: Udseende (Panelernes uigennemsigtighed (%), Farveskema, Layout, Sprog), Redigering, Visning og Brugervejledning.
- **Spil**: spillet, hvor dets filer ligger, og dit navn til samarbejde. Et spil angiver, hvilket kortformat der skal skrives, hvilke entitetsdefinitioner der skal indlæses, og hvor dets materialer ligger. Spilkonfigurationer læses fra mappen `games` i konfigurationsmappen (`~/.config/heron-brush/games` på Linux og macOS, `%LOCALAPPDATA%\heron-brush\games` på Windows) og fra mappen `games` ved siden af programmet, hvor pakkerne lægger den medfølgende konfiguration.
- **Visning**: Layout, Farveskema, Sprog, verdensakser, gitterets uigennemsigtighed, cellestørrelse for lækagesøgning og placeringslåsene.
- **Farver**: farverne for visninger, geometri og håndtag.
- **Mus**: hastighed og invertering for at se, panorere og flytte, flyvehastighed, synsfelt, Sammenkæd 2D-visninger og håndtagsstørrelser.
- **Tastatur**: tastaturgenvejene, Heron Brush eller Blender.
- **Renderer**: realistisk skygning, markeringsomrids, lys, skygger, ambient occlusion, refleksioner, bloom og overflader.

## Sprog
**Sprog** tilbyder Følg skrivebordet og alle oversatte sprog. Brugerfladen skifter med det samme. Manualen åbner på brugerfladens sprog, når der findes en oversættelse af den, og ellers på engelsk.

# Plugins og MCP
## Plugins
Plugins udvider editoren med Rhai-scripts. Fil → Plugins åbner vinduet Plugins. **Installer et plugin** installerer en `.rhai`-fil, og **Installer en plugin-mappe** installerer en mappe, der indeholder `main.rhai`; valgfri `plugin.toml`-metadata beskriver pluginet og dets indstillinger. Plugins er grupperet under **Aktive plugins** og **Inaktive plugins**, og **Slå plugin til** slår et til eller fra; det bevares efter en genstart. **Genindlæs plugins** læser scriptsene fra disken igen og nulstiller deres panelværdier.

Aktive plugins vises også i sektionen **Plugins** under fanen Kort med deres indstillinger, handlingsknapper (**Kør**) og output. En handling virker på markeringen, eller på alt, når intet er markeret, og er ét fortrydelsestrin. Den medfølgende Selection inspector viser antal og handlinger til at flytte op og ned.

## MCP
Samarbejde → Server for en agent (MCP) lader en AI-agent eller et andet værktøj styre den åbne editor via Model Context Protocol. Vinduet viser den lokale adresse med **Kopiér**, og **Stop server** afslutter den; mens den kører, viser titellinjen **MCP aktiv**. Kommandolinjeindstillingen `--mcp` kører den samme grænseflade uden vindue. Klienter kan finde værktøjer til geometri, assets, egenskaber, animation, eksport og sessioner. MCP og editor-plugins er forskellige grænseflader.

# Kommandolinje
Programmet hedder `heron-brush`. Uden indstillinger åbner det velkomstskærmen.

## Åbning af filer
`heron-brush map.map [entities.fgd] [materials/]` åbner et kort, eventuelt med entitetsdefinitioner og en materialemappe. Et tomt argument springer et over, for eksempel `heron-brush map.map "" materials/`. Hvis en editor allerede kører, åbner kortet i den; `--new-instance` åbner i stedet et separat vindue.

## Kommandoer
- `--help` eller `-h`: udskriv brugsanvisningen. `--version` eller `-V`: udskriv versionen.
- `--mcp`: server MCP over standardinput og -output, uden vindue.
- `--merge-map <base> <ours> <theirs> [<output>]`: en trevejsfletning af `.map`-filer til git. Outputtet er som standard ours. Konflikter udskrives og giver en afslutningskode, der ikke er nul.
- `--obj-to-map <in.obj> <out.map> [size] [thickness] [snap] [material]`: lav et mesh om til brushes. Size er længden af den længste side i kortenheder (standard 256), snap er et gitter for hjørnerne (0 for intet), og ordene `solid` eller `layered` bygger gitterkasser eller konvekse plader i stedet for tynde kiler.
- `--render-preview <in.map> <out.png> [size]`: skriv en PNG-miniature af et kort, 512 pixel som standard.
- `--run-script <script.rhai> <in.map> [out.map] [key=value ...]`: kør et plugin-script på et kort med alt markeret, og skriv resultatet (som standard tilbage til inputtet). `key=value` besvarer pluginets indstillinger.

# Fejlfinding
## Når noget ser forkert ud
- Manglende materiale: kontrollér materialemappen eller -pakken og eksportens projektstier.
- Forkert modelstørrelse: sammenlign med spillermarkøren på 64 enheder og eksportskalaen.
- Forkert gradientretning: kontrollér gradientvinklen og fladens orientering.
- Tom motorscene: læs eksportadvarslerne, og følg målets importvejledning.
- Live-forbindelsen venter: kontrollér, at tilføjelsen er installeret, at scenen er tilknyttet, og at opdateringer ikke er sat på pause.
- Kun linjer, ingen flader: slå Trådmodel fra i sektionen Visning under fanen Kort, eller vælg Materialer eller Massiv visning i 3D-vinduets menu.
- Objekter mangler i visningerne: `Ctrl+Shift+I` viser skjulte objekter; kontrollér lagene og View filters.
- En tast gør ingenting: klik først i visningen, så den har tastaturet, og kontrollér tastaturgenvejene under Indstillinger → Tastatur.
- En redigering mislykkedes: fortryd, forenkl geometrien, og brug **Kontrollér kort** og **Søg efter huller** i sektionen **Problemer** under fanen Kort.
- Et kort vil ikke åbne fra filhåndteringen: det kan være blevet overdraget til en editor, der allerede kører; kig efter det vindue.

## Indstillinger og sikkerhedskopier
Indstillinger, seneste kort, stjerner, eksportindstillinger og plugins ligger i konfigurationsmappen: `~/.config/heron-brush` på Linux og macOS og `%LOCALAPPDATA%\heron-brush` på Windows. Autosave-sikkerhedskopier ligger i mappen `autosave` ved siden af hvert kort.

## Rapportering af en fejl
Angiv editorens version (Hjælp → Om), dit styresystem, et lille kort, der viser problemet, og de præcise trin. Outputruden (`Ctrl+4`) fortæller ofte, hvad der gik galt. Gem en kopi, før du eksperimenterer.

## Brug af denne manual
Vælg et kapitel i indholdsfortegnelsen til venstre. `Ctrl+F` viser indholdsfortegnelsen og går til søgefeltet; `Esc` rydder søgningen eller lukker manualen, når søgningen er tom. **Indhold** skjuler eller viser kapitellisten, og **Tekststørrelse** ændrer skriftstørrelsen; begge huskes. Sproglisten øverst skifter mellem de tilgængelige oversættelser.
