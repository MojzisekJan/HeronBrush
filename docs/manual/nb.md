# Kom i gang
Heron Brush er et brush-basert banedigeringsprogram. Du bygger baner av konvekse brusher, maler og teksturerer flatene deres, plasserer entiteter, animerer dører og plattformer og eksporterer resultatet til en spillmotor. Programmet har også innebygde ressurser, parametriske former, live-koblinger til motorenes redigeringsprogrammer, delte redigeringsøkter og rendering av stillbilder. Denne brukerveiledningen følger med programmet og fungerer uten nett.

På macOS fungerer `Cmd` overalt der denne veiledningen sier `Ctrl`, og Option er Alt.

## Velkomstskjermen
Når Heron Brush starter uten et kart, åpnes velkomstskjermen. Til venstre finner du **Fortsett** (det siste kartet du hadde åpent), **Nytt kart** og **Bla gjennom** for å åpne en eksisterende `.map`, samt **Spill**, **Brukerveiledning** og **Innstillinger**. Til høyre er en liste over nylige kart med miniatyrbilder, etterfulgt av **Eksempler**. Stjernen (**Hold øverst**) fester et kart øverst i listen; søppelbøtta (**Flytt til papirkurven**) flytter filen til systemets papirkurv, slik at den kan gjenopprettes. Piltastene flytter rundt på skjermen, og Enter åpner elementet som har fokus. Fil → Velkommen henter fram skjermen igjen når som helst.

## Ditt første kart
1. Velg **Nytt kart** på velkomstskjermen, eller Fil → Nytt kart (`Ctrl+N`).
2. Velg Brushverktøyet (`B`) i verktøylinjen og dra i en 2D-visning for å tegne den valgte formen. Kontroller dybden i en annen 2D-visning.
3. Merk brushen og flytt den ved å dra. Bruk fanen **Flate** for å gi den et materiale eller en farge.
4. Lagre kildekartet med `Ctrl+S`. Når du er klar til å bruke det i et spill, åpner du Fil → Eksporter og skriver en scene for motoren din.

## Filer og lagring
`.map`-filen er den redigerbare kilden din. Lagre jevnlig: eksport, live-koblinger og rendering lagrer aldri kartet for deg. Fil → Lagre som (`Ctrl+Shift+S`) legger til filendelsen `.map` hvis du utelater den. Fil → Gå tilbake til lagret kart forkaster ulagrede endringer og laster inn filen på nytt. Hvis du lukker et vindu eller åpner et annet kart mens det finnes ulagrede endringer, spør Heron Brush om du vil **Lagre**, **Forkast** eller **Avbryt**.

Så lenge et lagret kart har ulagrede endringer, skrives nummererte sikkerhetskopier hvert tiende minutt til en `autosave`-mappe ved siden av kartet. De 50 nyeste sikkerhetskopiene beholdes.

Åpner du et kart mens Heron Brush allerede kjører, overtas det av det kjørende vinduet, som kommer fram i forgrunnen. Bruk `--new-instance` på kommandolinjen når du vil ha et eget vindu (se Kommandolinje).

# Grensesnittet
Hovedvinduet har en tittellinje med menyene, en verktøylinje under den, kartvisningene, inspektøren til høyre og en statuslinje nederst.

## Tittellinje og menyer
Tittellinjen viser logoen, menyene, kartnavnet i midten og vindusknappene. På macOS sitter de vanlige røde, gule og grønne knappene til venstre. Dra i tittellinjen for å flytte vinduet, og dobbeltklikk på den for å maksimere. Mens noe kjører i bakgrunnen, vises en indikator nær enden av linjen: en live-kobling til en motor, en live-økt eller **MCP aktiv**. Klikk på en indikator for å stoppe live-koblingen eller åpne vinduet dens.

- **Fil**: Velkommen, Nytt kart (`Ctrl+N`), Åpne kart (`Ctrl+O`), Lagre (`Ctrl+S`), Lagre som (`Ctrl+Shift+S`), Gå tilbake til lagret kart, Eksporter, Render bilde, Programtillegg og Innstillinger (`Ctrl+,`).
- **Rediger**: Angre (`Ctrl+Z`), Gjør om (`Ctrl+Shift+Z`), Klipp ut, Kopier, Lim inn, Dupliser (`Ctrl+D`), Slett, Velg alt (`Ctrl+A`), Gjenta siste kommandoer (`Ctrl+R`) og Glem gjentakbare kommandoer.
- **Vis**: 3D-visning i eget vindu, Gå til, Fest til rutenett (`Alt+0`), Vis rutenett (`0`), Forhåndsvis animasjoner og Vis utelatte flater.
- **Geometri**: Sett inn 3D-tekst, Sammenføyd hjørne, Konverter flater til patcher, Flytt utvalget ut av entiteten og Synkroniser koblede grupper.
- **Animasjon**: Nøkkelbildeanimasjon, Forhåndsvis animasjoner og Plasser kamera fra 3D-visningen (`Ctrl+Alt+0`).
- **Kart**: Last inn, last inn på nytt og lukk punktfil; last inn, last inn på nytt og lukk portalfil. Disse viser lekkasjespor og portaler skrevet av kartkompilatorer.
- **Samarbeid**: Live-økt og Server for en agent (MCP).
- **Hjelp**: Brukerveiledning og Om.

## Verktøylinjen
Verktøylinjen inneholder alle verktøyene. Den første gruppen brukes hele tiden: Velg, Brush, Klipp, Hjørnepunkt, Kant, Flate, Roter og Skaler. Den andre gruppen inneholder Skjær, Sett sammen, Sveip, Kontrollpunkt, Avrunding og Mal. Hold pekeren over en knapp for å se navnet og hurtigtasten. Klikker du på det aktive verktøyet igjen, går du tilbake til Velg.

Etter verktøyene kommer **Speilvend vannrett** og **Speilvend loddrett**, deretter to brytere: **Lås** beholder materialtilordningen når en brush flyttes, og **UV** beholder den når en brush roteres (`U` slår den av og på). Deretter følger innstillingene for det aktive verktøyet: Brushverktøyet viser formen og alternativene for den, og Avrundingsverktøyet viser radius, flater og hjørner med **Bruk**. Helt til høyre står verktøynavnet, rutenettstørrelsen (**Rutenett 16** osv.; klikk for å velge en annen) og menyen **Rutenettinnstillinger** med Fest til rutenett, Vis rutenett, Forhåndsvis animasjoner, Vis utelatte flater og Nøkkelbildeanimasjon.

## Kartvisninger
Det finnes fire visninger: 3D-visningen og tre flate visninger, Ovenfra (XY), Forfra (XZ) og Side (YZ). Innstillinger → Vis → Oppsett velger mellom Én rute, To ruter, Tre ruter og Fire ruter. Dra i skillelinjene for å endre størrelsen på rutene. Klikker du i en visning, får den tastaturet. `Space` setter en rute som viser én visning om gangen over til neste visning, og `Ctrl+Space` maksimerer visningen som har tastaturet, og gjenoppretter oppsettet igjen.

Høyreklikk uten å dra i en hvilken som helst visning for å åpne en hurtigmeny for utvalget og det som ligger under pekeren. Den tilbyr gruppering, lag, koblede grupper, oppretting av entiteter, flytting av brusher inn i eller ut av en entitet, kopiering av et materialnavn og visning av det i materialleseren.

## Inspektøren
Inspektøren til høyre har fire faner. Dra i venstrekanten for å endre bredden.

- **Kart** (`Ctrl+1`): Lag, Grupper, Programtillegg, Problemer, View filters og Merker.
- **Entitet** (`Ctrl+2`): entitetsleseren, entitetsdefinisjonene, egenskapene til den valgte entiteten og, for knapper og utløsere, Animasjonsutløser.
- **Flate** (`Ctrl+3`): UV, Materialer, Farge, Sveip, Avrunding, Patch og Innfelling og avfasing. Hver seksjon kan felles sammen.
- **Ressurser**: biblioteket med gjenbrukbare brush-modeller.

`Ctrl+5` skjuler eller viser inspektøren, og `Ctrl+Alt+T` skjuler eller viser verktøylinjen, slik at visningene får mer plass.

## Statuslinjen og utdataruten
Statuslinjen nederst viser den siste meldingen fra redigeringsprogrammet. Feil vises i rødt. Klikk på linjen, eller trykk `Ctrl+4`, for å åpne utdataruten med hele meldingshistorikken. De fleste kommandoer rapporterer her hva de gjorde, for eksempel hvor mange brusher som ble klippet.

## Dialoger og utseende
Dialoger som Innstillinger, Eksporter og Render bilde åpnes i egne vinduer. Innstillinger → Vis → Fargetema velger mellom Følg skrivebordet, Lyst og Mørkt. **Panelets dekkevne (%)** på siden Oversikt gjør panelene gjennomsiktige over visningene. Vinduet tegner sine egne knapper i systemets stil: innebygd på macOS, Windows 11-stil på Windows og GNOME-stil på Linux.

# Navigasjon og tastatur
## Bevege seg i 3D-visningen
Hold inne høyre museknapp og dra for å se deg rundt. Dra med midtknappen for å panorere. Hjulet flytter fram og tilbake. `W`, `A`, `S` og `D` flyr framover, til venstre, bakover og til høyre, `Q` stiger og `X` synker; hold `Shift` for større steg. Når **Alt + dra med midtknappen flytter kameraet** er slått på i Innstillinger → Mus, beveger Alt med midtknappen seg langs visningsretningen. Hastigheter, inverteringer og synsfeltet finner du i Innstillinger → Mus.

## Bevege seg i 2D-visninger
Dra med midtknappen for å panorere, og bruk hjulet for å zoome rundt pekeren. Panorering og zooming i én flat visning holder de andre på linje; **Koble 2D-visninger** i Innstillinger → Mus styrer dette. `Ctrl+Alt+Z` tilbakestiller zoomen, og `Ctrl+Shift+U` tilbakestiller de flate visningene til origo.

`Ctrl+U` rammer inn utvalget i alle visninger. Vis → Gå til (`Ctrl+Alt+M`) flytter kameraet til inntastede koordinater, velger det som står på en linje i kartfilen, eller flytter utvalget med en inntastet avstand.

## Rutenett og festing
`+` og `-` endrer rutenettstørrelsen, og talltastene `1` til `9` velger en størrelse direkte, fra 1 til 256 enheter. `0` eller `#` viser eller skjuler rutenettet, og `Alt+0` slår festing av eller på. `S` fester hjørnepunktene til de valgte brushene til gjeldende rutenett, `Ctrl+Shift+V` fester dem til hele enheter og `Ctrl+Alt+Shift+V` til rutenettet.

## Vanlige hurtigtaster
- `Ctrl+S`: lagre kartet. `Ctrl+Shift+S`: lagre som.
- `Ctrl+Z`: angre. `Ctrl+Shift+Z` eller `Ctrl+Y`: gjør om.
- `Ctrl+C`, `Ctrl+X`, `Ctrl+V`: kopier, klipp ut og lim inn. Lim inn plasserer objektene ved pekeren; `Ctrl+Alt+V` beholder de opprinnelige koordinatene. Når flater er valgt, kopierer kopier materialet og UV-innstillingene deres.
- `Ctrl+D`: dupliser. `Ctrl+Shift+D`: koblet duplikat av en gruppe.
- `Delete` eller `Backspace`: slett utvalget.
- `Esc`: avbryt den pågående dra-operasjonen, gå et steg tilbake i det aktive verktøyet, gå tilbake til Velg og til slutt tøm utvalget. `Shift+Esc` går direkte tilbake til Velg.
- `Ctrl+R`: gjenta de siste gjentakbare kommandoene. `Ctrl+Shift+R`: glem dem.
- `Ctrl+Alt+0` eller `Ctrl+Alt+Num0`: plasser renderkameraet ved 3D-visningen.
- `F5`: last inn materialene på nytt. `F6`: last inn entitetsdefinisjonene på nytt.

## Hurtigtaster for utvalg
- `Ctrl+A`: velg alt, eller alle håndtak mens et hjørnepunkt-, kant- eller flateverktøy er aktivt. `Ctrl+Shift+A`: velg ingenting. `Ctrl+Alt+A`: inverter utvalget.
- `Ctrl+B`: velg de andre objektene i samme entitet eller gruppe.
- `Ctrl+T`: velg alt som de valgte brushene berører. `Ctrl+E`: velg alt inne i dem. `Ctrl+Shift+E`: velg alt i søylen deres langs visningsaksen. Disse forbruker de velgende brushene.
- `Ctrl+I`: isoler utvalget. `Ctrl+Alt+I`: skjul det. `Ctrl+Shift+I`: vis alt igjen.
- `Ctrl+G`: grupper. `Ctrl+Shift+G`: opphev gruppering.

## Hurtigtaster for transformering
- Piltastene og `Page Up` / `Page Down` flytter utvalget ett rutenettsteg i retningen de peker i den aktive visningen. Med `Ctrl` dupliserer de mens de flytter, og med `Alt` roterer de.
- `Alt+X`, `Alt+Y`, `Alt+Z`: roter en kvart omdreining om den aksen; legg til `Shift` for motsatt retning.
- `Ctrl+F`: speilvend på tvers av visningen; `Ctrl+Alt+F`: speilvend opp-ned. `Ctrl+Shift+X` og `Ctrl+Shift+Y` speiler langs X eller Y.
- `Insert`: plasser en punktentitet i midten av utvalget.
- `Tab`: med en brush valgt, gå i ring gjennom Velg → Hjørnepunkt → Kant → Flate → Velg.

## Blender-tastaturoppsettet
Innstillinger → Tastatur → Tastaturoppsett tilbyr Heron Brush (standard, brukt gjennom hele denne veiledningen) og Blender. Med Blender: `G` velger og flytter, `R` roterer, `S` skalerer, `E` sveiper, `1`, `2` og `3` velger hjørnepunkt-, kant- og flateverktøyet, `K` klipper, `B` velger, `A` velger alt og `Alt+A` fjerner utvalget, `X` sletter, `Shift+D` dupliserer, `H` skjuler og `Alt+H` viser alt, og `Ctrl+Shift+Z` gjør om. Alt annet er likt i begge.

# Brusher og modelleringsverktøy
Brusher er konvekse legemer definert av plan. Bygg sammensatte objekter av flere enkle brusher i stedet for å prøve å gjøre én brush konkav. Hold antallet brusher lavt der en enkel form er nok.

## Tegne brusher
Velg Brushverktøyet (`B`) og dra i en visning. Verktøylinjen viser **Form som skal tegnes**: Rett prisme, Trapp, Hvelving, Sylinder, Kjegle, Kule (UV), Kule (ikosaeder) og Sett inn 3D-tekst. `Alt+1` til `Alt+7` velger en form direkte. Avhengig av formen viser linjen **Hul** og **Tykkelse:**, **Trinnhøyde:** og **Retning:** for trapper, **Akse:**, **Sider:** og sirkelmodusen for runde former, **Ringer:** for kuler og **Nøyaktighet:** for ikosaederet. Angi alternativene før du drar. Kontroller resultatet i mer enn én visning.

## Velge
Klikk på en brush for å velge den, og `Ctrl`-klikk for å legge til i utvalget. Drar du fra et tomt område, tegnes en utvalgsramme. I 3D-visningen velger `Shift`-klikk én enkelt flate, og `Ctrl+Shift`-klikk legger til flere flater. Drar du en valgt brush, flyttes den; drar du en som ikke er valgt, velges og flyttes bare den.

I 3D-visningen kan du `Shift`-dra en flate på en valgt brush for å ekstrudere den. `Alt`-klikk på en annen flate for å kopiere materialet og justeringen fra den valgte flaten over på den; `Alt+Ctrl` kopierer bare materialet, og `Alt+Shift` kopierer rotasjonen.

## Formingsverktøy
- **Velg** (`Esc`): velg og flytt.
- **Hjørnepunkt** (`V`), **Kant** (`E`), **Flate** (`F`): dra hjørner, kanter eller flater. `Shift`-dra fra midten av en kant eller flate for å legge til et hjørne der. `Delete` fjerner de valgte hjørnene.
- **Roter** (`R`), **Skaler** (`T`), **Skjær** (`G`): transformer utvalget med håndtak.
- **Klipp** (`C`): klikk to eller tre punkter for å definere et snittplan. `Enter` klipper, `Ctrl+Enter` velger hvilken side som skal beholdes, og `Backspace` eller `Esc` fjerner det siste punktet.
- **Sett sammen** (`N`): plasser punkter rundt et volum og trykk `Enter` for å lage en konveks brush av dem. `Backspace` fjerner det siste punktet.
- **Sveip** (`Y`): se avsnittet om sveip nedenfor.
- **Kontrollpunkt** (`P`): endre formen på kontrollnettet til en patch.
- **Avrunding** (`O`): klikk på en kant for å runde den av, eller velg kanter eller hjørnepunkter med verktøyene deres og trykk `Enter`. Flere flater gir et jevnere resultat og mer geometri.
- **Mal** (`K`): se Materialer og UV-er.

## Kombinere brusher
- `Ctrl+J`: slå sammen de valgte brushene til deres konvekse hylster.
- `Ctrl+K`: trekk de valgte brushene fra brushene de berører.
- `Ctrl+Shift+K`: gjør utvalget hult, med vegger som er ett rutenettsteg tykke.
- `Ctrl+L`: finn snittet av utvalget.

## Sveip, avrunding, patch og avfasing
**Sveip** bygger geometri langs en bane ut fra valgte flater. Velg flatene (i Sveip-verktøyet med `Alt`-klikk), angi deretter **Bevegelse**, **Rotasjon** (akse og vinkel), **Skaler**, **Trinn** og **Bane** (justering Heltall eller Fri; Bue, Rett eller S-kurve) i Sveip-seksjonen på fanen Flate, og trykk **Sveip**. Linjen under kontrollene forteller hvor mange brusher som vil bli laget.

Seksjonen **Avrunding** runder av de valgte kantene med **Radius**, **Flater** og **Hjørner** (Gjæret eller Avrundet); én flate gir en avfasing.

**Patch** viser **Kolonner** og **Rader** for den valgte patchen, med knapper for å legge til eller fjerne et par. Geometri → Konverter flater til patcher (`Ctrl+P`) lager patcher av valgte flater.

**Innfelling og avfasing** virker på valgte flater: **Innfelling** trekker flaten innover fra kantene, og **Dybde** skyver den ut (eller inn, når verdien er negativ). **Avfas flater** bruker begge; den tredje knappen legger til et panel eller skjærer ut en fordypning, avhengig av fortegnet til Dybde. **Sammenføyd hjørne** åpner en dialog som føyer sammen to brusher i et gjæret eller avrundet hjørne, med forhåndsvisning.

## 3D-tekst
Geometri → Sett inn 3D-tekst (også det siste elementet i formlisten til Brushverktøyet) bygger brusher av tekst. Den bruker Heron Block, en innebygd polygonskrift med avskårne hjørner, proporsjonale bredder og tsjekkiske aksenter; små bokstaver bruker store bokstavformer. Skriv inn **Tekst**, angi **Høyde** (versalhøyden; aksentene stiger over den) og **Dybde**, følg med på den direkte forhåndsvisningen og trykk **Sett inn**.

Hvis en operasjon ikke kan lage gyldig geometri, juster målene, eller angre og forenkle formen.

## Arrayer
Geometri → Array kopierer utvalget og viser kopiene i kartet mens du endrer verdiene. Antallene inkluderer originalen, og et array lager høyst 1024 kopier.

- **Rekke** flytter hver kopi i forhold til den forrige med **Steg X**, **Steg Y** og **Steg Z** og dreier den **Dreiing per kopi (grader)** om den valgte aksen.
- **Rutenett** fyller **Antall X**, **Antall Y** og **Antall Z** elementer med avstanden **Steg X**, **Steg Y** og **Steg Z**.
- **Ring** fordeler elementene over **Vinkel (grader)** rundt et sentrum **Radius (enheter)** fra utvalget. Med **Vend mot sentrum** dreier hver kopi med ringen.
- **Spredning** plasserer kopiene på tilfeldige steder innenfor radien, på tvers av den valgte aksen. Samme **Seed** gir samme plassering, og **Tilfeldig dreiing** dreier hver kopi.

Når utvalget er én hel gruppe, gjør **Koblede kopier** hver kopi til et koblet duplikat, så en endring i én når alle. **Bruk** beholder kopiene og velger dem sammen med originalen; **Avbryt** fjerner dem. Hele arrayet angres i ett steg.

## Modellere terreng
Terreng består av mange små brushes som modelleringspenselen skyver ut og trekker inn overflaten på. Velg én eller flere brushes og velg Geometri → Del i søyler. Brushene deles langs rutenettlinjene i søyler som er ett rutenettsteg brede, og søylene legges i en gruppe kalt Terrain. Snittene følger rutenettet i verden, så brushes som ligger inntil hverandre, deles på de samme stedene og deler hjørner. Hver søyle står langs den tynne siden av brushen: loddrett i et gulv eller et tak, vannrett i en vegg. En hule lages på samme måte: gjør en boks hul (Hollow selection) med veggtykkelse på ett rutenettsteg, velg veggene og del dem.

Modelleringslinjen velger formen på søylene. **Trekant** lager to søyler per rute, og toppen av en trekant forblir plan uansett hvordan den modelleres. **Kvadrat** lager én per rute, altså halvparten så mange brushes; toppen bøyer seg langs en diagonal.

Velg verktøyet **Modeller** (`J`) og dra over overflaten i 3D-visningen. Den oransje ringen viser penselen. Uten utvalg endrer penselen alle brushes under den; med et utvalg bare de valgte. Linjen øverst i vinduet velger hva et strøk gjør:

- **Hev** og **Senk** flytter overflaten ut eller inn, mest midt i ringen og ikke i det hele tatt ved kanten. Hold `Shift` for å bytte dem.
- **Glatt** jevner ut kuler.
- **Jevn ut** planerer overflaten til der strøket begynte.

**Retning** velger hvilken vei:

- **Opp** flytter overflaten rett opp og ned, innenfor en sirkel på bakken. Dette er for terreng.
- **Kamera** flytter den mot og bort fra visningen, innenfor en kule rundt pekeren. Se på et gulv, en vegg eller et tak, så former penselen det du ser på.
- **Normal** flytter den ut fra flaten under pekeren der strøket begynner.

De to tallene er **Radius (enheter)** og **Styrke (%)**. Bare hjørnene på den siden strøket beveger, flytter seg; baksiden blir der den er, og en søyle blir minst én enhet tykk. Der brushes deler et hjørne, flytter de det sammen, så overflaten forblir lukket. Et strøk angres i ett steg. I 2D-visningene velger verktøyet som valgverktøyet.

# Materialer og UV-er
## Laste inn og bruke materialer
Åpne seksjonen **Materialer** på fanen Flate. **Last inn mappe** legger til en mappe med bilder, og **Last inn .wad** legger til en teksturpakke; begge vises under **Samlinger**, der krysset fjerner en av dem. Skriv i **Filtrer materialer** for å snevre inn listen. Klikk på et materiale for å gjøre det gjeldende, og **Bruk på utvalget** legger det deretter på alle valgte flater. **Velg** tilbyr Velg brusher med det og Velg flater med det. **Erstatt** bytter ut ett materiale med et annet i hele kartet eller bare i utvalget.

Verktøyet **Mal** (`K`) legger det gjeldende materialet på flaten under pekeren: et klikk maler én flate, et dobbeltklikk maler hele brushen, og `Shift`-klikk henter flatens materiale som det gjeldende, som en pipette.

## UV-redigering
Seksjonen **UV** på fanen Flate viser materialet og omrisset til den valgte flaten. Musekontroller på lerretet:
- Hjulet zoomer rundt pekeren. Dra med høyre knapp eller midtknappen for å panorere. Dobbeltklikk med høyre knapp for å ramme inn flaten igjen; velger du en annen flate, rammes den inn automatisk.
- Dra med venstre knapp for å flytte materialet mens flatens omriss blir stående. Ringen roterer tilordningen, hjørnehåndtakene skalerer den, og midthåndtaket angir origo for disse endringene.
- `Alt`-dra med venstre knapp på kanten av en flis for å skjære, det samme gjør `Shift`-dra med midtknappen.
- Hver dra-operasjon er ett angresteg, og `Esc` avbryter en dra-operasjon som pågår.

Under lerretet finner du **Forskyvning**, **Skaler**, **Vinkel** og, når spillet bruker dem, **Innholdsflagg**, **Overflateflagg** og **Overflateverdi**. Knappene er **Tilbakestill**, **Tilbakestill til verden**, **Tilpass**, **Speilvend H**, **Speilvend V** og de to 90°-rotasjonene. **Juster** dreier materialet langs en av flatens kanter (trykk igjen for neste kant, `Shift` for forrige). De fire justeringsknappene skyver materialet inntil en kant. **Tilpass U** og **Tilpass V** skalerer slik at et helt antall kopier dekker flaten; trykk igjen for neste antall, og hold `Ctrl` for å beholde inndelingen i et trim sheet.

## UV-taster
Når flater er valgt, flytter piltastene materialet, `[` og `]` dreier det 15°, `,` og `.` halverer eller dobler skalaen, `Ctrl+F` og `Ctrl+Alt+F` speilvender det, `Shift+R` tilbakestiller det, `Alt+Shift+R` tilbakestiller det til verdensaksene, og `Ctrl+Shift+F` tilpasser det til hver flate.

**Lås** i verktøylinjen beholder tilordningen mens brusher flyttes, og **UV** (`U`) beholder den mens brusher roteres.

## Verktøymaterialer
- skip: fjerner den merkede flaten fra motoreksporter.
- clip: lager kollisjon uten synlig geometri.
- origin: markerer pivotpunktet til en brush-entitet.

Navnene kan endres på kortet **Verktøymaterialer** i Eksporter-vinduet. Bruk skip for flater du vet er skjult.

## Utelate flater fra eksport
Velg en flate og slå på **Utelat flaten fra eksport** i UV-seksjonen. Oransje skravering markerer den i UV-forhåndsvisningen. Materialet og malingen blir lagret, og brushen forblir lukket og redigerbar. Eksporterte rendermesher utelater den flaten, mens kollisjonen beholdes. Utelater du én firkant av en kube, blir det to rendertrekanter. Dette gjør ikke flater tosidige og slår ikke sammen mesher.

**Utelat tildekkede flater** merker flater som er helt dekket av statiske brusher, også identiske flater som berører hverandre og innesluttede brusher. Det virker på de valgte brushene, eller på all synlig, ulåst verdensgeometri når ingenting er valgt. Brush-entiteter, gjennomsiktige materialer og brusher som allerede har utelatte flater, hoppes over. Delvis dekkede flater deles ikke. Ett angresteg gjenoppretter alt. Vis → Vis utelatte flater skjuler eller viser slike flater i visningene i redigeringsprogrammet; plukking bruker fortsatt hele brushen.

# Farge og gradienter
Seksjonen **Farge** på fanen Flate maler flater med ensfargede farger eller gradienter oppå materialene deres. Det er nyttig for å skissere en bane og for stilisert grafikk. Malte flater beholder sine virkelige farger i visningene; utvalget vises med omrisset.

## Palett og blandingsfarge
Klikk på en av de tolv fargeprøvene i paletten for å male de valgte flatene, eller hele valgte brusher, med den. **Dekkevne** angir hvor dekkende neste maling er, i prosent. **Bland** har en fargeknapp som åpner fargevelgeren; dra i velgeren, så følger utvalget med direkte, eller trykk **Mal** for å bruke blandingsfargen som ett steg. **Tilbake til materialet** fjerner malingen, slik at flaten viser materialet sitt igjen. Linjen øverst i seksjonen forteller hva som er valgt og hva det har på seg.

## Fargevelgeren
Alle fargeknapper i redigeringsprogrammet åpner den samme velgeren. Dra i kvadratet for å velge metning og lysstyrke, og i stripen ved siden av for å velge fargetone. Under finner du verdiene **R**, **G** og **B** og en glidebryter for **Dekkevne** der fargen har en. Heksadesimalfeltet tar `#RRGGBB` eller `#RRGGBBAA`; det blir rødt så lenge teksten ikke er en gyldig farge. **Kopier** legger heksadesimalkoden på utklippstavlen. Klikk utenfor velgeren for å lukke den.

## Gradienter
**Gradient** velger Lineær gradient eller Radiell gradient, og knappen ved siden av bytter om fargene fra ende til ende. Hvert stopp har en fargeknapp og **Posisjon (%)**, og stopp mellom endene har en fjernknapp. **Legg til fargestopp** legger til et stopp i det største mellomrommet; en gradient kan ha opptil åtte stopp. **Gradientvinkel** angir retningen til en lineær gradient, med knappene **+45°** og **−45°**.

Under forhåndsvisningen er en skinne med en markør for hvert stopp. Klikk på skinnen for å legge til et stopp, klikk på en markør for å åpne velgeren dens, dra en markør for å flytte den, og høyreklikk på en markør for å fjerne den. De tre fargeprøvene under skinnen er forhåndsinnstillinger: Varm gradient, Kald gradient og Gråtonegradient. Endringer i en gradient brukes på utvalget direkte.

Retningen til en gradient betyr noe: legg den langs en takflate eller langs et buet objekt, ikke på tvers av flater som ikke hører sammen. Kontroller modellen fra begge sider etter malingen.

## Overflatemateriale
**Overflatemateriale** angir **Ruhet** (0 er blankt, 100 er matt), **Metallisk** (0 for tre, stein eller plast, 100 for metall) og **Emisjon** (0 til 8; overflaten gløder i sin egen farge, også gradienter, men lyser ikke opp objekter i nærheten). Endringene brukes direkte på de valgte flatene eller brushene. **Bruk standardoverflate** fjerner overstyringen. Alle endringer kan angres.

Overflateeffekter vises i den opplyste 3D-visningen og i kamerarenderinger. Flat visning og trådmodellvisning utelater høylysene med vilje. Glød kan legge til et skjær rundt selvlysende overflater. Godot og glTF/GLB mottar materialverdiene, og Unity-importøren gjør ruhet om til glatthet. Se `docs/surface-materials.md` for begrensningene til hvert mål.

# Ressurser, lag og grupper
## Ressursbiblioteket
Åpne fanen **Ressurser**. Legg til en mappe med `.map`-modeller med **Legg til mappe**; det innebygde biblioteket er alltid der. Skriv i **Filtrer ressurser** for å søke. Klikk på en modell for å velge den, og dobbeltklikk deretter på den, trykk `Enter` eller klikk **Sett inn** for å legge den til der pekeren sist var. Du kan også dra en modell inn i en visning. Ressursbrusher settes inn som én gruppe, slik at modellen flyttes samlet.

**Lagre utvalget som ressurs** skriver de valgte brushene til biblioteket som en `.map`. Ressurser i biblioteket er vanlige redigerbare brusher, ikke importerte mesher.

## Lag
Listen **Lag** på fanen Kart organiserer store områder. Skriv inn et navn og klikk **Legg til** for å lage et lag, eller trykk `Enter` for å gi det gjeldende laget nytt navn. Hvert lag kan skjules, låses eller utelates fra eksport. **Flytt hit** flytter utvalget til det gjeldende laget. Hurtigmenyen i visningene tilbyr også Flytt til lag, Gjør til aktivt lag, Skjul lag og Isoler lag.

## Grupper og koblede grupper
`Ctrl+G` grupperer utvalget, og `Ctrl+Shift+G` opphever grupperingen. Seksjonen **Grupper** på fanen Kart viser gruppene og tilbyr **Åpne**, **Gi nytt navn**, **Slå sammen**, **Opphev gruppering**, **Ta ut** og **Trekk ut**. En koblet gruppe er et sett med kopier som deler endringer: `Ctrl+Shift+D` lager et koblet duplikat, Geometri → Synkroniser koblede grupper oppdaterer kopiene, og **Fjern kobling** gjør en kopi uavhengig. På fanen Entitet kan en egenskap beholdes som en kopis egen når koblede grupper synkroniseres.

## En felles skala
Standard eksportskala er 32 kartenheter per meter. Den innebygde startmarkøren for spilleren er 64 enheter høy, en referanse på 2 meter. Sammenlign døråpninger, kjøretøy, møbler og planter med den. Endre eksportskalaen bare med vilje, og hold beslektede ressurser konsekvente.

# Entiteter og spillmekanikk
Vanlige entitetsdefinisjoner er innebygd og alltid tilgjengelige: lys, startpunkter for spillere, dører, knapper, plattformer, utløsere, statiske rekvisitter, lyder og veipunkter. Laster du inn en FGD for prosjektet med **Entitetsdefinisjoner** på fanen Entitet, erstattes klasser med samme navn, mens de andre innebygde beholdes.

## Plassere entiteter
I leseren **Entiteter** på fanen Entitet skriver du i **Filtrer klasser** og velger en klasse. For en punktklasse, for eksempel et lys, klikker du **Plasser** eller dobbeltklikker på klassen; `Insert` plasserer en i midten av utvalget. For en brush-klasse, for eksempel en dør eller en utløser, velger du brushene og klikker **Lag**. Klasser kan også dras inn i en visning.

## Egenskaper
Egenskapslisten redigerer den valgte entiteten. Entiteter av samme klasse redigeres sammen. **Legg til egenskap** legger til en nøkkel som definisjonen kanskje ikke kjenner; **Standardverdier** kan sette eksisterende egenskaper tilbake til standardverdiene eller legge til de manglende. Egenskaper som klassen definerer, men som entiteten ikke har, vises nedtonet; skriver du inn en verdi, legges de til. Bruk targetname og target for å koble entiteter sammen.

## Hva en eksport kan gjøre
Godots direktiver lager innebygde noder, kollisjonslegemer og utløserområder. Unity og Unreal støtter vanlige lys som innebygde lys, og Unreal lager også PlayerStart-aktører. Andre data beholdes så langt målets importør tillater.

En dørentitet alene gir ingen spilloppførsel. Bevegelse, reaksjoner på utløsere, spillerkontrollere og prosjektskript må fortsatt lages i motoren. Defold eksporterer entitetsegenskaper og kollisjonsobjekter; Flax plasserer modeller og generiske entiteter. Kontroller resultatet i motoren før du bygger spillmekanikk på det.

# Animasjon
Heron Brush kan forhåndsvise enkel bevegelse og redigere nøkkelbildeanimasjoner av brush-entiteter, og deretter eksportere dem til flere motorer.

## Forhåndsvise animasjoner
Slå på **Forhåndsvis animasjoner** i menyen Vis, menyen Animasjon, menyen Rutenettinnstillinger eller Vis-seksjonen på fanen Kart. Den spiller av rotasjonen til func_rotate og fram-og-tilbake-bevegelsen til func_shuttle når start_on er slått på, ved hjelp av entitetens egenskaper for akse, hastighet, retning, bevegelse og ventetid, og den spiller av nøkkelbildeklipp som er satt til Spill av ved start. Vifte- og vindmølleressursene har allerede roterende deler.

Når du slår av forhåndsvisningen, gjenopprettes den opprinnelige stillingen. Et klikk i en visning stopper avspillingen før noe velges eller redigeres; kameranavigeringen fungerer fortsatt. Forhåndsvisningen endrer ikke kartet, angrehistorikken eller eksportene, og den kjører verken fysikk eller spillogikk.

## Nøkkelbilderedigering
Åpne den med Animasjon → Nøkkelbildeanimasjon eller fra menyen Rutenettinnstillinger. Den forankres under visningene, eller i 3D-vinduet mens det er separat. Vindusikonet i overskriften flytter den til et eget vindu; den beholder bildet og angrehistorikken. **Rediger animasjon** setter visningene over til den animerte stillingen, og **Rediger kart** går tilbake til hvilegeometrien. I animasjonsmodus viser hver visning **Rediger animasjon** og bildenummeret nederst.

Velg en brush-entitet, eller en brush eller brushgruppe, for å animere den. En flat brushgruppe deler én animasjon og ett pivotpunkt; grupper som inneholder flere entiteter, trenger separate animasjoner.

## Klipp
Velgeren **Animasjon** bytter mellom navngitte klipp. Skriv inn et navn i **Animasjonsnavn** og klikk **Lagre som** for å kopiere det gjeldende klippet under det navnet; **Slett** fjerner et klipp. **Flytt** lager et `translate`-klipp uten rotasjon. **Døranimasjoner** lager jevne `open`- og `close`-klipp med en dreining på 90 grader rundt det gjeldende pivotpunktet; flytt pivotpunktet til hengselen først. **Spill av ved start** får klippet til å spilles av når banen starter; dørforhåndsinnstillingen slår det av.

## Transport og avspilling
Transportknappene er **Start**, **Forrige nøkkel**, **Avspilling**, **Neste nøkkel** og **Slutt**. **Bilde** viser og angir det gjeldende bildet, og **Varighet (s)** angir lengden på klippet. Modusen er **Én gang**, **Løkke** eller **Frem og tilbake**.

## Tidslinjen
Tidslinjen har en linjal med avspillingshodet og et spor for **Posisjon** (X, Y og Z med felles nøkler) og for hver rotasjonsvinkel, RX, RY og RZ. Klikk eller dra i linjalen for å flytte avspillingshodet og forhåndsvise den stillingen i alle visninger. Dra med midtknappen for å panorere tidslinjen. Dra nøkler for å flytte dem i tid; `Shift` legger til i utvalget; dobbeltklikk på et spor for å sette inn en nøkkel; `Ctrl`+hjul zoomer. **Kurver** viser verdikurvene i stedet for sporene.

Knappene over den er **Legg til nøkkel**, **Fjern**, **Kopier**, **Lim inn** og **Dupliser**. **Fest til bilder** holder nøklene på hele bilder, og **Tilpass** viser hele klippet. **Endre tidspunkt for nøkler** åpner **Tidsskala**, **Forskyvning (bilder)**, **Skaler fra avspillingshodet**, **Forleng varigheten** og **Endre tidspunkt for utvalget**.

Nøkkelinspektøren ved siden av tidslinjen redigerer den valgte nøkkelen: **Bilde**, **Verdi** (eller de tre koordinatene til en posisjonsnøkkel), interpolasjonen **Lineær**, **Konstant** eller **Bézier**, **Innkommende helning** og **Utgående helning**, og **Glatte tangenter**.

## Taster i redigeringen
Mens tidslinjen har tastaturet: `Space` spiller av eller setter på pause, venstre og høyre piltast går ett bilde, `Ctrl+A` velger alle nøkler, `Ctrl+C`, `Ctrl+X` og `Ctrl+V` kopierer, klipper ut og limer inn nøkler, `Ctrl+D` dupliserer dem, `Delete` fjerner dem, `Ctrl+Z` angrer og `Esc` avbryter en dra-operasjon. Utenfor tidslinjen går `Esc` ut av animasjonsmodus. Angre beholder zoomen, utvalget og avspillingshodet der de var.

## Stille opp i visningene
Raden under tidslinjen har **Løkskall** med et antall og **Avstand**, som viser stillinger i nærheten som spøkelser; koordinatene for **Pivotpunkt**; **Automatisk nøkkel**; og stillingsverktøyene **Flytt**, **Roter** og **Pivotpunkt**.

- **Legg til nøkkel** slår på Automatisk nøkkel og setter avspillingen på pause. Med Automatisk nøkkel på og **Flytt** valgt kan du dra en farget akse på håndtaket til den valgte entiteten i 3D-visningen for å sette en nøkkel for posisjonen ved det gjeldende bildet.
- I en 2D-visning kan du dra den valgte brushen eller et medlem av gruppen dens for å sette nøkler for forflytning langs de to synlige aksene, med festing til rutenettet. Automatisk nøkkel trenger ikke være på for dette.
- Med **Roter** drar du brushen i en 2D-visning rundt den oransje pivotmarkøren. Ovenfra setter nøkler for RZ, Forfra for RY og Side for RX, festet til rutenettvinkler.
- Med **Pivotpunkt** drar du det gule rotasjonssenteret i en hvilken som helst 2D-visning. Det festes til rutenettet og beholder dybden. Nøkler og hvilegeometri forblir uendret.

Den turkise banen i 2D og 3D viser hvordan pivotpunktet beveger seg. Klikk på et banepunkt for å velge tidspunktet og nøklene, eller dra det for å endre posisjonen; med Automatisk nøkkel på kan du også dra banepunkter i 3D-visningen. Etiketter viser bildenumre, og stiplede segmenter markerer hopp. `Esc` avbryter alle disse dra-operasjonene, og ett angresteg omgjør en hel dra-operasjon.

Nye animasjoner starter i hvilestilling med nøkler ved tidspunkt null. Lag bevegelse ved å endre stillingen ved et annet bilde. For en skarp L-formet bane setter du nøkler for starten, hjørnet og slutten, og bruker Lineær med Frem og tilbake.

## Koble knapper til animasjoner
Velg en func_button, trigger_multiple eller trigger_once. Fanen Entitet viser **Animasjonsutløser** over egenskapene. Velg et mål fra listen, eller klikk **Velg mål i visningen** og deretter et animert objekt i en hvilken som helst visning; `Esc` avbryter valget. Manglende mål eller klipp rapporteres under kontrollene. Entitetskoblinger viser piler til målene sine når visning av koblinger er slått på.

**Handling** velger hva aktivering gjør: **Spill av** starter et navngitt klipp eller den valgte animasjonen; **Veksle** veksler mellom `open` og `close` når de finnes, ellers starter den den valgte animasjonen på nytt; **Åpne** og **Lukk** starter de klippene; **Pause** holder stillingen, **Fortsett** fortsetter, og **Stopp** gjenoppretter den opprinnelige stillingen. **Bare én gang** overstyrer klassens standard. **Gjentakelsesforsinkelse (s)** angir korteste tid mellom aktiveringer; den forsinker ikke den første.

**Test utløser** kjører handlingen i forhåndsvisningen uten å redigere kartet eller legge til angresteg. **Tilbakestill forhåndsvisning** gjenoppretter de opprinnelige stillingene og nullstiller tilstanden for én gang og nedkjøling. Dette simulerer verken spillerkollisjoner eller spillskript.

## Animasjon i eksporter
Godot eksporterer innebygde AnimationPlayer- og AnimationLibrary-ressurser, inkludert kurver og bevegelig kollisjon. En CharacterBody3D som går inn i området til en knapp, aktiverer målet dens. Endringer du gjør i disse klippene i Godot, kommer ikke tilbake til kartet og erstattes ved neste eksport.

Raylib-eksporten legger til `<map>_animation.h` med en avspiller i C og `<map>_preview.c` med en forhåndsvisning med fritt kamera; følg `<map>_raylib.md` for å bygge den. I forhåndsvisningen setter `SPACE` på pause, `R` starter standardklippene på nytt og `N` går gjennom de navngitte klippene. Defold bygger avspillingen inn i entitetsskriptet med dokumenterte `heron_*_animation`-meldinger. Build Level i Unity legger til `HeronBrushAnimation` og et kinematisk legeme. Bevy-eksporten inneholder `heron_animation.rs` og instruksjoner. Godot, Unity, Defold, Bevy og raylib beholder utløserkoblingene, én gang-flagget og gjentakelsesforsinkelsen. Spillet ditt må fortsatt registrere avspilleren og koble sine egne knapper til API-et for hver kjøretid.

## Eksempelet Motion lab
Åpne `motion_lab` fra Eksempler på velkomstskjermen. Det har en plattform med L-bane, en heis, en roterende plattform og en hengslet dør med knapper. Åpne `godot/project.godot` ved siden av det i Godot og trykk `F5`. WASD og musen flytter spilleren, Space hopper, 1 til 4 besøker stasjonene og R starter den gjeldende stasjonen på nytt. Tråkk på en farget plate for å aktivere målet dens. Den medfølgende README-filen forklarer ny eksport.

# Eksportere til en motor
Fil → Eksporter åpner Eksporter-vinduet. Motorene står i en liste ned langs venstre side med logoene sine: Godot, Defold, three.js, Bevy, raylib, Unity, Unreal Engine, Flax Engine, Cocos Creator, Wavefront .obj, glTF .gltf, GLB .glb og til slutt `.map`. Siden til høyre forteller hva målet vil få. **Skriv** skriver det, **Kontroller** bygger banen uten å skrive den og viser problemene, og **Lukk** lukker vinduet.

Eksport er atskilt fra lagring: `.map` forblir den redigerbare kilden din. Hver kontroll lagrer innstillingene for motoren sin så snart den endres, slik at hver motor beholder sine egne svar og vinduet åpnes slik du forlot det.

## Vanlige alternativer
- **Skaler**: **Enheter per meter**, 32 som standard.
- **Hva som skal skrives**: **Kollisjonsgeometri**.
- **Optimaliseringer**: **Grupper statiske mesher i celler på 1024 enheter**, **Fjern flater som ligger begravd i geometrien**, **Slå sammen flater som deler plan** og **Slå sammen kollisjonsformer**.
- **Prosjektstier**: hvor banen og ressursene dens havner i prosjektet.
- **Verktøymaterialer**: navnene på materialene clip, skip og origin.
- **Mer**: motorspesifikke brytere som **Navngi noder etter**, **Gjenoppbygg lag og grupper som foreldernoder**, **Snu trekantenes omløpsretning**, speilvending av teksturkoordinater og et ekstra UV-sett for bakt lys med **Lightmap-texel**.
- **Holder følge med kartet**: **Skriv på nytt hver gang kartet lagres**. Dette blir tilgjengelig etter at motoren er eksportert én gang, fordi den eksporten forteller hvor det skal skrives.

Statiske celler tilordner verdensbrusher etter sentrum og slår dem sammen etter materiale innenfor hver celle på 1024 enheter, slik at motorer kan utelate mindre deler av kartet fra tegning. Kollisjonen beholdes, og spillentiteter holdes atskilt. Det kan gi flere tegnekall enn én verdensmesh, så det er slått av som standard. Frittstående OBJ-, glTF- og GLB-eksporter bruker ikke celler.

## Merknader om målene
- Godot: `res://`-stier for teksturer og materialer, det fysiske laget for utløsere og lightmap-UV2. Velg en eksportmappe i Godot-prosjektet.
- Defold: ressursmappen for ressurser, modellmaterialet og teksturutvalgeren, som må finnes i det valgte materialet.
- Unity: OBJ-mesher og en importør for redigeringsprogrammet; eksporter under prosjektets Assets-mappe.
- Unreal Engine: et generert importskript og en innholdssti, vanligvis /Game/<level>.
- Flax Engine: en innholdssti for modeller relativt til prosjektets Content-mappe.
- three.js, Bevy, raylib og Cocos Creator: scener eller kode for disse kjøretidene, med instruksjoner ved siden av eksporten.
- glTF / GLB: flyttbar geometri for kompatible verktøy; GLB er den binære formen. OBJ: geometri og materialer for modelleringsverktøy.
- `.map`: **Skriv som** velger Valve 220, Standard, Quake 2 eller Quake 2 (Valve). Vinduet forteller hva den valgte dialekten ville miste.

Les advarslene etter skrivingen, for eksempel om manglende teksturer. **Hva kontrollen fant** viser dem i vinduet.

# Live-koblinger: Godot, Unity, Defold og Flax
En live-kobling sender ulagrede endringer fra Heron Brush til redigeringsprogrammet for en motor mens du arbeider. Den går én vei: endringer gjort i motoren kommer ikke tilbake. Én live-kobling kan kjøre per kartvindu. Stopp den før du bytter motor, og start den på nytt for å ta i bruk endrede eksportalternativer. Kontrollene finner du i Eksporter-vinduet, på siden for motoren, på kortet **Live-kobling med**.

Indikatoren i tittellinjen viser tilstanden til koblingen; klikk på den for å stoppe koblingen. Lukker eller erstatter du kartet, stopper publiseringen. Etter en importfeil retter du problemet og starter koblingen på nytt.

## Godot
1. Eksporter én gang til et Godot-prosjekt.
2. Klikk **Installer Godot-tillegget**.
3. Slå på Heron Brush Live i Godots Project Settings → Plugins.
4. Klikk **Start live-kobling** og koble kartet fra Godot-panelet til en åpen 3D-scene.

Tillegget erstatter bare sin egen genererte gren og beholder plasseringen av tilkoblingen og andre noder. Lagre Godot-scenen selv, og hold dine egne skript og noder utenfor den genererte grenen.

## Unity
1. Eksporter én gang under Unity-prosjektets Assets-mappe.
2. Klikk **Installer Unity-tillegget** og la Unity kompilere det.
3. Åpne Window → Heron Brush → Live Link i Unity.
4. Start koblingen og koble kartet til den aktive Unity-scenen.

Hold Unitys live-vindu åpent. Oppdateringer settes på pause i Play Mode og Prefab Mode. Bare underobjektet Generated i kartobjektet erstattes. Ikke rediger de genererte filene.

## Defold
1. Eksporter én gang til Defold-prosjektet ditt og hold prosjektet åpent i Defold.
2. Klikk **Start live-kobling**. Ingen tillegg trengs.
3. Åpne heron_brush_live/preview.collection i Defold, eller referer til den fra din egen scene.

Koblingen skriver til en egen generert mappe og lar den endelige eksporten og game.project være i fred. **Last inn det kjørende spillet på nytt** oppdaterer også et spill startet fra Defold; å legge til objekter eller endre kollisjon kan kreve at banen startes på nytt. Mislykkede tilkoblinger prøves på nytt automatisk.

## Flax
Eksporter til prosjektets Content-mappe og klikk **Installer Flax-tillegget**. La Flax kompilere det, start koblingen, og velg deretter Tools → Heron Brush: attach live map i den åpne Flax-scenen. Den genererte roten beholder plasseringen sin, og andre objekter røres ikke. Lagre scenen selv. Oppdateringer settes på pause i Play Mode. Tools → Heron Brush: stop live map setter mottaket på pause.

# Samarbeid i live-økter
Samarbeid → Live-økt åpner et vindu for å være vert for eller bli med hos en annen Heron Brush-bruker. Dette deler kartredigering mellom personer; det er atskilt fra en live-kobling til en motor.

## Være vert og bli med
Skriv inn **Navnet ditt**. For å være vert velger du **Vær vert for en økt**, klikker **Start økt** og sender detaljene fra **Kopier tilkoblingsdetaljer** til gjestene dine. For å bli med velger du **Bli med i en økt**, limer inn detaljene og klikker **Bli med**. Når du blir med, erstattes det åpne kartet med vertens kart, så lagre arbeidet ditt først.

Standardversjonen fungerer i det lokale nettverket, og **Port i lokalt nettverk** angir porten. Økter over internett (**Koble til over internett**) krever en versjon med den valgfrie nettverksfunksjonen. Deltakerne ser hverandres kameramarkører og navn i 3D-visningen.

Lukker du vinduet, fortsetter økten å kjøre; indikatoren i tittellinjen viser tilstanden. **Stopp økten** eller **Forlat økten** avslutter den. Når to personer redigerer samme objekt, avgjør vertens rekkefølge den endelige versjonen, og angring er også en delt endring. Arbeid i separate områder der du kan, og lagre kontrollpunkter.

# 3D-visning og en ekstra skjerm
## Visningsmoduser
Seksjonen **Vis** på fanen Kart bytter hva visningene tegner: Brusher, Punktentiteter, Entitetsmodeller, Entitetsklassenavn, Omriss av grupper, Patcher, Materialer, Vis utelatte flater, Forhåndsvis animasjoner, Trådramme, Kanter og Omriss av brush-entiteter. Trådramme tegner trekantkanter, også diagonaler og kanter bak overflater, uten materialer. Det er en midlertidig feilsøkingsvisning og påvirker ikke eksporter eller renderinger.

Belysning, skygger, omgivelsesokklusjon, refleksjoner, glød og utjevning for 3D-visningen finner du i Innstillinger → Renderer. Dette er innstillinger for forhåndsvisningen; ferdige bilder bruker renderkameraets egne innstillinger.

## Et eget 3D-vindu
Vis → 3D-visning i eget vindu flytter 3D-visningen til et eget vindu, som du kan plassere på en annen skjerm. Det deler kameraet, dokumentet og verktøyene, og de flate visningene tar plassen i hovedvinduet. Menyen tilbyr Materialer, Heldekkende visning og Trådramme; en Vis-undermeny med Vis utelatte flater, Forhåndsvis animasjoner, Kanter, Omriss av grupper, Punktentiteter, Entitetsmodeller og Omriss av brush-entiteter; Fokuser på utvalget; Forfra, Side og Ovenfra, som dreier kameraet rundt utvalget (eller et punkt foran kameraet); Plasser kamera fra 3D-visningen; Fullskjerm (`F11`); Render bilde; og Tilbake til hovedvinduet. Lukker du vinduet, forankres visningen igjen.

## Prosedyrisk himmel
Den prosedyriske himmelen er slått på som standard i 3D-visningen og kan slås av i Innstillinger → Renderer → Lys. Renderkameraer har sin egen bryter for **Prosedyrisk himmel**, som lagres med kartet. Fargene på himmel, horisont og bakke kommer fra miljøinnstillingene, og solen følger belysningen i scenen. Himmelen har ingen geometri og eksporteres ikke.

# Renderkameraer og PNG-bilder
Fil → Render bilde åpner renderingsvinduet: kameraet og innstillingene til venstre og en direkte forhåndsvisning til høyre som tegnes på nytt mens du endrer dem. Kameraer lagres i `.map`-filen i et Render cameras-lag som bare finnes i redigeringsprogrammet og utelates fra motoreksporter.

## Plassere kameraet
Komponer bildet i 3D-visningen og trykk `Ctrl+Alt+0` (eller `Ctrl+Alt+Num0`), eller klikk **Plasser kamera fra 3D-visningen**. Hvis ingen kamera er valgt, oppdateres det første lagrede; hvis ingen finnes, opprettes ett. Dette kan angres og beholder innstillingene for utdata og belysning. **Velg kameraet i kartet**, **Last inn kamera fra kartet** og **Flytt 3D-visningen til kameraet** virker motsatt vei. **Kameraramme i 3D-visning** viser grensen for utdataene, og **Styr kameraet fra 3D-visningen** lar vanlig navigering flytte det lagrede kameraet.

## Innstillinger
- **Kameraposisjon**: X, Y, Z, giring og stigning.
- **Utdata og objektiv**: **Forhåndsinnstillinger** (Full HD, Kvadrat, Stående), bredde, høyde, samples, **Vertikalt synsfelt** og **Fjernt klippeplan**.
- **Belysning**: Dagslys, Solnedgang eller Overskyet, eller angi eksponering, solen, fargene på himmel, horisont og bakke, **Prosedyrisk himmel**, **Skygger**, **Omgivelsesokklusjon** og **Glød**.
- **Dybdeskarphet**: **Slå på dybdeskarphet**, fokusavstand, skarpt område og uskarphetsradius. **Fokuser på utvalget** fokuserer på midten av et valgt objekt foran kameraet.

**Lagre kamera og innstillinger** lagrer skjemaet i kartet; lagre kartet for å beholde det på disken.

## Rendering
**Render PNG** spør hvor filen skal lagres, og starter en progressiv rendering, som fylles ut sample for sample i forhåndsvisningen. Flere samples ser bedre ut, men tar lengre tid. **Avbryt rendering** stopper uten å skrive en fil. Rendereren bruker rasterpipelinen til redigeringsprogrammet, ikke strålesporing. PNG-filen har en ugjennomsiktig bakgrunn, og animasjoner renderes ikke. Svært store bilder kan stoppe redigeringsprogrammet et kort øyeblikk.

# Innstillinger
Fil → Innstillinger (`Ctrl+,`) åpner innstillingene. Endringer trer i kraft umiddelbart og lagres. Sidepanelet viser Oversikt, Spill, Vis, Farger, Mus, Tastatur og Renderer. Søkefeltet øverst finner en innstilling etter navn eller beskrivelse på alle sider. Hver endret innstilling har sin egen knapp for å gjenopprette standardverdien, og **Gjenopprett standardverdier** tilbakestiller den gjeldende siden.

## Sidene
- **Oversikt**: de vanlige innstillingene samlet: Utseende (Panelets dekkevne (%), Fargetema, Oppsett, Språk), Redigering, Vis og Brukerveiledning.
- **Spill**: spillet, hvor filene til det ligger, og navnet ditt for samarbeid. Et spill bestemmer hvilket kartformat som skrives, hvilke entitetsdefinisjoner som lastes og hvor materialene ligger. Spillkonfigurasjoner leses fra `games`-mappen i konfigurasjonsmappen (`~/.config/heron-brush/games` på Linux og macOS, `%LOCALAPPDATA%\heron-brush\games` på Windows) og fra `games`-mappen ved siden av programmet, der pakkene legger den medfølgende konfigurasjonen.
- **Vis**: Oppsett, Fargetema, Språk, verdensakser, rutenettets dekkevne, cellestørrelse for lekkasjekontroll og låsene for materialtilordning.
- **Farger**: fargene på visningene, geometrien og håndtakene.
- **Mus**: hastigheter og invertering for å se, panorere og flytte, flyhastighet, synsfelt, Koble 2D-visninger og størrelse på håndtak.
- **Tastatur**: tastaturoppsettet, Heron Brush eller Blender.
- **Renderer**: realistisk skyggelegging, utvalgsomriss, lys, skygger, omgivelsesokklusjon, refleksjoner, glød og overflater.

## Språk
**Språk** tilbyr Følg skrivebordet og alle oversatte språk. Grensesnittet endres med en gang. Brukerveiledningen åpnes på grensesnittspråket når det finnes en oversettelse av den, og ellers på engelsk.

# Programtillegg og MCP
## Programtillegg
Programtillegg utvider redigeringsprogrammet med Rhai-skript. Fil → Programtillegg åpner vinduet Programtillegg. **Installer et programtillegg** installerer en `.rhai`-fil, og **Installer en programtilleggsmappe** installerer en mappe som inneholder `main.rhai`; valgfrie metadata i `plugin.toml` beskriver programtillegget og innstillingene for det. Programtillegg grupperes under **Aktive programtillegg** og **Inaktive programtillegg**, og **Slå på programtillegg** slår et av eller på; dette overlever en omstart. **Last inn programtillegg på nytt** leser skriptene fra disken igjen og tilbakestiller verdiene i panelene deres.

Aktive programtillegg vises også i seksjonen **Programtillegg** på fanen Kart, med innstillingene sine, handlingsknapper (**Kjør**) og utdata. En handling virker på utvalget, eller på alt når ingenting er valgt, og er ett angresteg. Den medfølgende Selection inspector viser antall og handlinger for å flytte opp og ned.

## MCP
Samarbeid → Server for en agent (MCP) lar en KI-agent eller et annet verktøy styre det åpne redigeringsprogrammet via Model Context Protocol. Vinduet viser den lokale adressen med **Kopier**, og **Stopp serveren** avslutter den; mens den kjører, viser tittellinjen **MCP aktiv**. Kommandolinjealternativet `--mcp` kjører det samme grensesnittet uten vindu. Klienter kan finne verktøy for geometri, ressurser, egenskaper, animasjon, eksport og økter. MCP og programtillegg i redigeringsprogrammet er forskjellige grensesnitt.

# Kommandolinje
Programmet heter `heron-brush`. Uten alternativer åpner det velkomstskjermen.

## Åpne filer
`heron-brush map.map [entities.fgd] [materials/]` åpner et kart, eventuelt med entitetsdefinisjoner og en materialmappe. Et tomt argument hopper over ett, for eksempel `heron-brush map.map "" materials/`. Hvis et redigeringsprogram allerede kjører, åpnes kartet i det; `--new-instance` åpner i stedet et eget vindu.

## Kommandoer
- `--help` eller `-h`: skriv ut bruken. `--version` eller `-V`: skriv ut versjonen.
- `--mcp`: server MCP over standard inn- og utdata, uten vindu.
- `--merge-map <base> <ours> <theirs> [<output>]`: en treveis sammenslåing av `.map`-filer for git. Utdata er som standard ours. Konflikter skrives ut og gir en avslutningskode som ikke er null.
- `--obj-to-map <in.obj> <out.map> [size] [thickness] [snap] [material]`: gjør en mesh om til brusher. Size er lengden på den lengste siden i kartenheter (standard 256), snap er et rutenett for hjørnene (0 for ingen), og ordene `solid` eller `layered` bygger rutenettbokser eller konvekse plater i stedet for tynne kiler.
- `--render-preview <in.map> <out.png> [size]`: skriv et PNG-miniatyrbilde av et kart, 512 piksler som standard.
- `--run-script <script.rhai> <in.map> [out.map] [key=value ...]`: kjør et programtilleggsskript på et kart med alt valgt, og skriv resultatet (som standard tilbake til inndatafilen). `key=value` besvarer innstillingene til programtillegget.

# Feilsøking
## Når noe ser feil ut
- Manglende materiale: kontroller materialmappen eller -pakken og prosjektstiene i eksporten.
- Feil modellstørrelse: sammenlign med spillermarkøren på 64 enheter og eksportskalaen.
- Feil gradientretning: kontroller gradientvinkelen og flatens orientering.
- Tom motorscene: les eksportadvarslene og følg importinstruksjonene for målet.
- Live-koblingen venter: kontroller at tillegget er installert, at scenen er koblet til og at oppdateringer ikke er satt på pause.
- Bare linjer, ingen flater: slå av Trådramme i Vis-seksjonen på fanen Kart, eller velg Materialer eller Heldekkende visning i menyen til 3D-vinduet.
- Objekter mangler i visningene: `Ctrl+Shift+I` viser skjulte objekter; kontroller lagene og View filters.
- En tast gjør ingenting: klikk i visningen først, slik at den har tastaturet, og sjekk tastaturoppsettet i Innstillinger → Tastatur.
- En redigering mislyktes: angre, forenkle geometrien, og bruk **Kontroller kart** og **Se etter lekkasjer** i seksjonen **Problemer** på fanen Kart.
- Et kart åpnes ikke fra filbehandleren: det kan ha blitt overlevert til et redigeringsprogram som allerede kjører; se etter det vinduet.

## Innstillinger og sikkerhetskopier
Innstillinger, nylige kart, stjerner, eksportinnstillinger og programtillegg ligger i konfigurasjonsmappen: `~/.config/heron-brush` på Linux og macOS, og `%LOCALAPPDATA%\heron-brush` på Windows. Automatiske sikkerhetskopier ligger i `autosave`-mappen ved siden av hvert kart.

## Rapportere en feil
Ta med versjonen av redigeringsprogrammet (Hjelp → Om), operativsystemet ditt, et lite kart som viser problemet, og de nøyaktige trinnene. Utdataruten (`Ctrl+4`) forteller ofte hva som gikk galt. Lagre en kopi før du eksperimenterer.

## Bruke denne veiledningen
Velg et kapittel i innholdsfortegnelsen til venstre. `Ctrl+F` viser innholdsfortegnelsen og flytter til søkefeltet; `Esc` tømmer søket, eller lukker veiledningen når søket er tomt. **Innhold** skjuler eller viser kapittellisten, og **Tekststørrelse** endrer skriftstørrelsen; begge huskes. Språklisten øverst bytter mellom de tilgjengelige oversettelsene.
