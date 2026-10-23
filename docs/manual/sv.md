# Kom igång
Heron Brush är en brush-baserad baneditor. Du bygger banor av konvexa brushar, målar och texturerar deras ytor, placerar entiteter, animerar dörrar och plattformar och exporterar resultatet till en spelmotor. Den har också inbyggda tillgångar, parametriska former, livelänkar till motorernas editorer, delade redigeringssessioner och rendering av stillbilder. Den här handboken följer med programmet och fungerar utan internetanslutning.

På macOS fungerar `Cmd` överallt där handboken skriver `Ctrl`, och Option motsvarar Alt.

## Välkomstskärmen
När Heron Brush startar utan karta öppnas välkomstskärmen. Till vänster finns **Fortsätt** (den senaste kartan du hade öppen), **Ny karta** och **Bläddra** för att öppna en befintlig `.map`, samt **Spel**, **Handbok** och **Inställningar**. Till höger finns en lista över senaste kartor med miniatyrer, följd av **Exempel**. Stjärnan (**Håll överst**) fäster en karta överst i listan; papperskorgen (**Flytta till papperskorgen**) flyttar filen till systemets papperskorg, så att den kan återställas. Piltangenterna flyttar runt på skärmen och Enter öppnar det markerade objektet. Arkiv → Välkommen tar fram skärmen igen när som helst.

## Din första karta
1. Välj **Ny karta** på välkomstskärmen, eller Arkiv → Ny karta (`Ctrl+N`).
2. Välj brushverktyget (`B`) i verktygsfältet och dra i en 2D-vy för att rita den valda formen. Kontrollera djupet i en annan 2D-vy.
3. Markera brushen och flytta den genom att dra. Använd fliken **Yta** för att ge den ett material eller en färg.
4. Spara källkartan med `Ctrl+S`. När du är redo att använda den i ett spel öppnar du Arkiv → Exportera och skriver en scen för din motor.

## Filer och sparande
`.map`-filen är din redigerbara källa. Spara ofta: export, livelänkar och rendering sparar aldrig kartan åt dig. Arkiv → Spara som (`Ctrl+Shift+S`) lägger till filändelsen `.map` om du utelämnar den. Arkiv → Återgå till sparad karta kastar osparade ändringar och läser in filen igen. Om du stänger ett fönster eller öppnar en annan karta med osparade ändringar frågar Heron Brush om du vill **Spara**, **Förkasta** eller **Avbryt**.

Medan en sparad karta har osparade ändringar skrivs numrerade säkerhetskopior var tionde minut till mappen `autosave` bredvid kartan. De 50 nyaste säkerhetskopiorna behålls.

Om du öppnar en karta medan Heron Brush redan körs lämnas den över till det fönster som körs, och det fönstret hamnar överst. Använd `--new-instance` på kommandoraden när du vill ha ett separat fönster (se Kommandoraden).

# Gränssnittet
Huvudfönstret har en namnlist med menyerna, ett verktygsfält under den, kartvyerna, inspektören till höger och ett statusfält längst ned.

## Namnlist och menyer
Namnlisten visar logotypen, menyerna, kartans namn i mitten och fönsterknapparna. På macOS sitter de vanliga röda, gula och gröna knapparna till vänster. Dra i namnlisten för att flytta fönstret och dubbelklicka på den för att maximera. Medan något körs i bakgrunden visas en indikator nära slutet av listen: en livelänk till en motor, en livesession eller **MCP aktivt**. Klicka på en indikator för att stoppa livelänken eller öppna dess fönster.

- **Arkiv**: Välkommen, Ny karta (`Ctrl+N`), Öppna karta (`Ctrl+O`), Spara (`Ctrl+S`), Spara som (`Ctrl+Shift+S`), Återgå till sparad karta, Exportera, Rendera bild, Tillägg och Inställningar (`Ctrl+,`).
- **Redigera**: Ångra (`Ctrl+Z`), Gör om (`Ctrl+Shift+Z`), Klipp ut, Kopiera, Klistra in, Duplicera (`Ctrl+D`), Ta bort, Markera allt (`Ctrl+A`), Upprepa senaste kommandon (`Ctrl+R`) och Glöm upprepningsbara kommandon.
- **Visa**: 3D-vy i separat fönster, Gå till, Fäst mot rutnät (`Alt+0`), Visa rutnät (`0`), Förhandsgranska animeringar och Visa utelämnade ytor.
- **Geometri**: Infoga 3D-text, Sammanfoga hörn, Konvertera ytor till patchar, Flytta markeringen ut ur sin entitet och Synka länkade grupper.
- **Animering**: Nyckelbildsanimering, Förhandsgranska animeringar och Placera kamera från 3D-vyn (`Ctrl+Alt+0`).
- **Karta**: Läs in, Läs in igen och Ta bort punktfil; Läs in, Läs in igen och Ta bort portalfil. De visar läckspår och portaler som kartkompilatorer har skrivit.
- **Samarbete**: Livesession och Tillhandahåll för en agent (MCP).
- **Hjälp**: Handbok och Om.

## Verktygsfältet
Verktygsfältet innehåller alla verktyg. Den första gruppen används hela tiden: Markera, Brush, Klipp, Hörnpunkt, Kant, Yta, Rotera och Skala. Den andra gruppen innehåller Skjuva, Montera, Svep, Kontrollpunkt, Avrundning och Måla. Håll pekaren över en knapp för att se dess namn och kortkommando. Om du klickar på det aktiva verktyget igen återgår du till Markera.

Efter verktygen kommer **Vänd horisontellt** och **Vänd vertikalt**, sedan två växlar: **Lås** behåller materialmappningen när en brush flyttas, och **UV** behåller den när en brush vrids (`U` växlar den). Därefter följer det aktiva verktygets egna inställningar: brushverktyget visar formen och dess alternativ, och avrundningsverktyget visar radie, ytor och hörn med **Verkställ**. Längst till höger finns verktygets namn, rutnätsstorleken (**Rutnät 16** och så vidare; klicka för att välja en annan) och menyn **Rutnätsinställningar** med Fäst mot rutnät, Visa rutnät, Förhandsgranska animeringar, Visa utelämnade ytor och Nyckelbildsanimering.

## Kartvyer
Det finns fyra vyer: 3D-vyn och tre platta vyer, Ovanifrån (XY), Framifrån (XZ) och Sida (YZ). Inställningar → Visa → Layout väljer En ruta, Två rutor, Tre rutor eller Fyra rutor. Dra i avdelarna för att ändra rutornas storlek. Ett klick i en vy ger den tangentbordet. `Space` växlar en ruta som visar en vy i taget till nästa vy, och `Ctrl+Space` maximerar vyn som har tangentbordet och återställer layouten igen.

Högerklicka utan att dra i valfri vy för att öppna en snabbmeny för markeringen och det som finns under pekaren. Den erbjuder gruppering, lager, länkade grupper, skapande av entiteter, flytt av brushar in i eller ut ur en entitet, kopiering av ett materialnamn och visning av det i materialbläddraren.

## Inspektören
Inspektören till höger har fyra flikar. Dra i dess vänstra kant för att ändra bredden.

- **Karta** (`Ctrl+1`): Lager, Grupper, Tillägg, Problem, View filters och Taggar.
- **Entitet** (`Ctrl+2`): entitetsbläddraren, entitetsdefinitionerna, egenskaperna för den markerade entiteten och, för knappar och utlösare, Animeringsutlösare.
- **Yta** (`Ctrl+3`): UV, Material, Färg, Svep, Avrundning, Patch samt Infällning & fasning. Varje avsnitt kan fällas ihop.
- **Tillgångar**: biblioteket med återanvändbara brushmodeller.

`Ctrl+5` döljer eller visar inspektören och `Ctrl+Alt+T` döljer eller visar verktygsfältet, vilket ger vyerna mer utrymme.

## Statusfältet och utdatarutan
Statusfältet längst ned visar editorns senaste meddelande. Fel visas i rött. Klicka på fältet, eller tryck på `Ctrl+4`, för att öppna utdatarutan med hela meddelandehistoriken. De flesta kommandon rapporterar här vad de gjorde, till exempel hur många brushar som klipptes.

## Dialogrutor och utseende
Dialogrutor som Inställningar, Exportera och Rendera bild öppnas i egna fönster. Inställningar → Visa → Färgschema väljer Följ skrivbordet, Ljust eller Mörkt. **Panelopacitet (%)** på sidan Översikt gör panelerna genomskinliga över vyerna. Fönstret ritar sina egna knappar i systemets stil: inbyggd stil på macOS, Windows 11-stil på Windows och GNOME-stil på Linux.

# Navigering och tangentbord
## Förflyttning i 3D-vyn
Håll ned höger musknapp och dra för att se dig omkring. Dra med mittknappen för att panorera. Hjulet flyttar framåt och bakåt. `W`, `A`, `S` och `D` flyger framåt, åt vänster, bakåt och åt höger, `Q` stiger och `X` sjunker; håll ned `Shift` för större steg. När **Alt + dra med mittknappen flyttar kameran** är påslaget i Inställningar → Mus flyttar Alt med mittknappen längs blickriktningen. Hastigheter, inverteringar och synfältet finns i Inställningar → Mus.

## Förflyttning i 2D-vyer
Dra med mittknappen för att panorera och använd hjulet för att zooma runt pekaren. När du panorerar och zoomar en platt vy hålls de andra i linje; **Länka 2D-vyer** i Inställningar → Mus styr detta. `Ctrl+Alt+Z` återställer zoomen och `Ctrl+Shift+U` återställer de platta vyerna till origo.

`Ctrl+U` ramar in markeringen i alla vyer. Visa → Gå till (`Ctrl+Alt+M`) flyttar kameran till inskrivna koordinater, markerar det som finns på en rad i kartfilen eller flyttar markeringen en inskriven sträcka.

## Rutnät och fästning
`+` och `-` ändrar rutnätsstorleken, och sifferknapparna `1` till `9` väljer en storlek direkt, från 1 till 256 enheter. `0` eller `#` visar eller döljer rutnätet, och `Alt+0` slår på eller av fästning. `S` fäster de markerade brusharnas hörnpunkter mot det aktuella rutnätet, `Ctrl+Shift+V` fäster dem mot hela enheter och `Ctrl+Alt+Shift+V` mot rutnätet.

## Vanliga kortkommandon
- `F3` eller `Ctrl+Shift+P`: sök efter valfritt menykommando med namn och kör det med `Enter`. `F10` öppnar menyerna, och piltangenterna flyttar i dem.
- `Ctrl+S`: spara kartan. `Ctrl+Shift+S`: spara som.
- `Ctrl+Z`: ångra. `Ctrl+Shift+Z` eller `Ctrl+Y`: gör om.
- `Ctrl+C`, `Ctrl+X`, `Ctrl+V`: kopiera, klipp ut och klistra in. Inklistring placerar objekten vid pekaren; `Ctrl+Alt+V` behåller deras ursprungliga koordinater. När ytor är markerade kopieras deras material och UV-inställningar.
- `Ctrl+D`: duplicera. `Ctrl+Shift+D`: länkad dubblett av en grupp.
- `Delete` eller `Backspace`: ta bort markeringen.
- `Esc`: avbryt den pågående dragningen, gå ett steg tillbaka i det aktiva verktyget, återgå till Markera och till sist rensa markeringen. `Shift+Esc` återgår direkt till Markera.
- `Ctrl+R`: upprepa de senaste upprepningsbara kommandona. `Ctrl+Shift+R`: glöm dem.
- `Ctrl+Alt+0` eller `Ctrl+Alt+Num0`: placera renderingskameran vid 3D-vyn.
- `F5`: läs in materialen igen. `F6`: läs in entitetsdefinitionerna igen.

## Kortkommandon för markering
- `Ctrl+A`: markera allt, eller alla handtag medan ett hörnpunkts-, kant- eller ytverktyg är aktivt. `Ctrl+Shift+A`: markera ingenting. `Ctrl+Alt+A`: invertera markeringen.
- `Ctrl+B`: markera de andra objekten i samma entitet eller grupp.
- `Ctrl+T`: markera allt som de markerade brusharna rör vid. `Ctrl+E`: markera allt inuti dem. `Ctrl+Shift+E`: markera allt i deras kolumn längs vyaxeln. De markerande brusharna förbrukas.
- `Ctrl+I`: isolera markeringen. `Ctrl+Alt+I`: dölj den. `Ctrl+Shift+I`: visa allt igen.
- `Ctrl+G`: gruppera. `Ctrl+Shift+G`: dela upp grupp.

## Kortkommandon för transformering
- Piltangenterna och `Page Up` / `Page Down` flyttar markeringen ett rutnätssteg i den riktning de pekar i den aktiva vyn. Med `Ctrl` dupliceras den medan den flyttas, och med `Alt` roteras den.
- `Alt+X`, `Alt+Y`, `Alt+Z`: rotera ett kvarts varv kring den axeln; lägg till `Shift` för motsatt riktning.
- `Ctrl+F`: vänd tvärs över vyn; `Ctrl+Alt+F`: vänd upp och ned. `Ctrl+Shift+X` och `Ctrl+Shift+Y` speglar längs X eller Y.
- `Insert`: placera en punktentitet i markeringens mitt.
- `Tab`: med en brush markerad växlar du Markera → Hörnpunkt → Kant → Yta → Markera.

## Blender-tangentbordsmappningen
Inställningar → Tangentbord → Tangentbordsmappning erbjuder Heron Brush (standard, som används i hela handboken) och Blender. Med Blender: `G` markerar och flyttar, `R` roterar, `S` skalar, `E` sveper, `1`, `2` och `3` väljer hörnpunkts-, kant- och ytverktyget, `K` klipper, `B` markerar, `A` markerar allt och `Alt+A` släpper markeringen, `X` tar bort, `Shift+D` duplicerar, `H` döljer och `Alt+H` visar allt, och `Ctrl+Shift+Z` gör om. Allt annat är likadant i båda.

# Brushar och modelleringsverktyg
Brushar är konvexa solider som definieras av plan. Bygg sammansatta objekt av flera enkla brushar i stället för att försöka göra en brush konkav. Håll nere antalet brushar där en enkel form räcker.

## Rita brushar
Välj brushverktyget (`B`) och dra i en vy. Verktygsfältet visar **Form att rita**: Rätblock, Trappa, Valvbåge, Cylinder, Kon, Sfäroid (UV), Sfäroid (ikosaeder) och Infoga 3D-text. `Alt+1` till `Alt+7` väljer en form direkt. Beroende på formen visar fältet **Gör ihålig** och **Tjocklek:**, **Steghöjd:** och **Riktning:** för trappor, **Axel:**, **Sidor:** och cirkelläget för runda former, **Ringar:** för sfärer och **Noggrannhet:** för ikosaedern. Ställ in alternativen innan du drar. Kontrollera resultatet i mer än en vy.

## Markera
Klicka på en brush för att markera den och `Ctrl`-klicka för att lägga till i markeringen. En dragning från tom yta ritar en markeringsruta. I 3D-vyn markerar `Shift`-klick en enskild yta och `Ctrl+Shift`-klick lägger till fler ytor. Om du drar en markerad brush flyttas den; om du drar en omarkerad brush markeras och flyttas bara den.

I 3D-vyn `Shift`-drar du en yta på en markerad brush för att extrudera den. `Alt`-klicka på en annan yta för att kopiera den markerade ytans material och justering till den; `Alt+Ctrl` kopierar bara materialet och `Alt+Shift` kopierar rotationen.

## Formverktyg
- **Markera** (`Esc`): markera och flytta.
- **Hörnpunkt** (`V`), **Kant** (`E`), **Yta** (`F`): dra hörn, kanter eller ytor. `Shift`-dragning från mitten av en kant eller yta lägger till ett hörn där. `Delete` tar bort de valda hörnen.
- **Rotera** (`R`), **Skala** (`T`), **Skjuva** (`G`): transformera markeringen med handtag.
- **Klipp** (`C`): klicka på två eller tre punkter för att definiera ett klipplan. `Enter` klipper, `Ctrl+Enter` väljer vilken sida som behålls, och `Backspace` eller `Esc` tar bort den senaste punkten.
- **Montera** (`N`): placera punkter runt en volym och tryck på `Enter` för att skapa en konvex brush av dem. `Backspace` tar bort den senaste punkten.
- **Svep** (`Y`): se avsnittet om svep nedan.
- **Kontrollpunkt** (`P`): forma om en patchs kontrollrutnät.
- **Avrundning** (`O`): klicka på en kant för att runda av den, eller välj kanter eller hörnpunkter med deras verktyg och tryck på `Enter`. Fler ytor ger ett mjukare resultat och mer geometri.
- **Måla** (`K`): se Material och UV.

## Kombinera brushar
- `Ctrl+J`: slå ihop de markerade brusharna till deras konvexa hölje.
- `Ctrl+K`: subtrahera de markerade brusharna från de brushar de rör vid.
- `Ctrl+Shift+K`: gör markeringen ihålig, med väggar som är ett rutnätssteg tjocka.
- `Ctrl+L`: bilda skärningen av markeringen.

## Svep, avrundning, patch och fasning
**Svep** bygger geometri längs en bana utifrån markerade ytor. Markera ytorna (i svepverktyget med `Alt`-klick), ställ sedan in **Sträcka**, **Vridning** (axel och vinkel), **Skala**, **Steg** och **Sökväg** (justering Heltal eller Fri; Båge, Rak eller S-kurva) i avsnittet Svep på fliken Yta, och tryck på **Svep**. Raden under kontrollerna anger hur många brushar det blir.

Avsnittet **Avrundning** rundar av de valda kanterna med **Radie**, **Ytor** och **Hörn** (Geringad eller Rundad); en yta ger en fas.

**Patch** visar den markerade patchens **Kolumner** och **Rader** med knappar för att lägga till eller ta bort ett par. Geometri → Konvertera ytor till patchar (`Ctrl+P`) skapar patchar av markerade ytor.

**Infällning & fasning** arbetar på markerade ytor: **Infällning** drar in ytan från dess kanter och **Djup** skjuter ut den (eller in, när värdet är negativt). **Fasa ytor** tillämpar båda; den tredje knappen lägger till en panel eller skär ut en fördjupning, beroende på tecknet för Djup. **Sammanfoga hörn** öppnar en dialogruta som sammanfogar två brushar i ett geringat eller rundat hörn, med förhandsvisning.

## 3D-text
Geometri → Infoga 3D-text (även sista posten i brushverktygets formlista) bygger brushar av text. Den använder Heron Block, ett inbyggt polygontypsnitt med avfasade hörn, proportionella bredder och tjeckiska diakritiska tecken; gemener använder versalformer. Skriv **Text**, ställ in **Höjd** (versalhöjden; diakritiska tecken sticker upp ovanför) och **Djup**, följ förhandsvisningen och tryck på **Infoga**.

Om en åtgärd inte kan skapa giltig geometri justerar du dess mått, eller ångrar och förenklar formen.

## Arrayer
Geometri → Array kopierar markeringen och visar kopiorna i kartan medan du ändrar värdena. Antalen räknar med originalet, och en array gör högst 1024 kopior.

- **Rad** flyttar varje kopia i förhållande till den förra med **Steg X**, **Steg Y** och **Steg Z** och vrider den **Vridning per kopia (grader)** kring den valda axeln.
- **Rutnät** fyller **Antal X**, **Antal Y** och **Antal Z** element med avståndet **Steg X**, **Steg Y** och **Steg Z**.
- **Ring** fördelar elementen över **Vinkel (grader)** runt ett centrum **Radie (enheter)** från markeringen. Med **Vänd mot centrum** vrids varje kopia med ringen.
- **Spridning** placerar kopiorna på slumpmässiga platser inom radien, tvärs över den valda axeln. Samma **Seed** ger samma placering, och **Slumpmässig vridning** vrider varje kopia.

När markeringen är en hel grupp gör **Länkade kopior** varje kopia till en länkad dubblett, så en ändring av en når alla. **Verkställ** behåller kopiorna och markerar dem tillsammans med originalet; **Avbryt** tar bort dem. Hela arrayen ångras i ett steg.

## Skulptera terräng
Terräng består av många små brushar vars yta skulpteringspenseln trycker ut och drar in. Markera en eller flera brushar och välj Geometri → Dela i pelare och sedan pelarnas form: Triangel, Kvadrat eller Sexhörning. Kommandot skär dem längs rutnätets linjer i pelare som är ett rutnätssteg breda och lägger pelarna i en grupp med namnet Terrain. Snitten följer världens rutnät, så brushar som ligger bredvid varandra skärs på samma ställen och delar hörn. Varje pelare står längs brushens tunna sida: upprätt i ett golv eller ett tak, liggande i en vägg. En grotta fungerar likadant: gör en låda ihålig (Hollow selection) med en väggtjocklek på ett rutnätssteg, markera väggarna och dela dem.

Fönstret **Skulptera**, som knappen i slutet av skulpteringslisten öppnar, kan också dela markeringen, i formen som är vald på dess rad Pelarform. Båda ställena minns den senast använda formen. **Triangel** ger två pelare per ruta i rutnätet, och en triangels ovansida förblir plan hur den än skulpteras. **Kvadrat** ger en per ruta, hälften så många brushar; dess ovansida böjs längs en diagonal. **Sexhörning** täcker brushen med sexhörningar två rutnätssteg breda, var och en delad i sex trianglar runt mitten, så att varje ovansida förblir plan.

Välj verktyget **Skulptera** (`J`) och dra över ytan i 3D-vyn. Håller du knappen nedtryckt utan att flytta musen fortsätter penseln att arbeta på samma ställe, tio gånger i sekunden, så att en kulle växer så länge du håller. Den orange ringen visar penseln. Utan markering ändrar penseln alla brushar under den; med en markering bara de markerade. Listen överst i fönstret väljer vad ett drag gör:

- **Höj** och **Sänk** flyttar ytan utåt eller inåt, mest i ringens mitt och inte alls vid kanten. Håll ned `Shift` för att byta dem.
- **Jämna ut** jämnar ut bulor.
- **Plana ut** planar ut ytan till där draget började.

I samma fönster väljer **Riktning** åt vilket håll:

- **Upp** flyttar ytan rakt upp och ned, inom en cirkel på marken. Det är till för terräng.
- **Kamera** flyttar den mot och från vyn, inom ett klot runt pekaren. Titta på ett golv, en vägg eller ett tak så formar penseln det.
- **Normal** flyttar den utåt från den yta som pekaren är över när draget börjar.

De två talen är **Radie (enheter)** och **Styrka (%)**. Bara hörnen på den sida som draget flyttar rör sig; baksidan stannar där den är och en pelare förblir minst en enhet tjock. Där brushar delar ett hörn flyttar de det tillsammans, så ytan förblir sluten. Ett drag ångras i ett steg. I 2D-vyerna markerar verktyget som markeringsverktyget.

När skulpteringen är klar markerar du terrängen och väljer Geometri → **Slå ihop pelare**, eller knappen i fönstret **Skulptera**. Grannpelare som tillsammans bildar en konvex brush och ser likadana ut blir en brush, så platta partier blir några få stora brushar. Ytan flyttar sig inte, och sammanslagningen ångras i ett steg.

För terräng som i voxelspel delar du i **Kvadrat**-pelare och väljer **Block** under **Drag** i fönstret **Skulptera**. Ett drag flyttar då hela ovansidan av varje pelare i hela rutnätssteg och håller sidorna lodräta, så marken stiger i trappsteg. Alla lägen fungerar som förut; små drag läggs ihop tills en pelare har stigit ett helt steg. Pelare med andra former lämnas orörda.

**Fritt** behöver inga pelare och fungerar på alla brushar. Höj lägger till mark i en kula runt pekaren och Sänk tar bort den, så marken kan få överhäng och grottor. Jämna ut jämnar ut den och Plana ut lägger den i nivå med där draget började. Varje tryck bygger om de berörda brusharna på rutnätet, avrundade, och lämnar allt utanför kulan som det var, skarpa kanter också. Resultatet är vanliga brushar, så alla andra verktyg fungerar på dem, och **Slå ihop pelare** gör dem färre efteråt. Ett finare rutnät ger en jämnare yta och fler brushar.

# Material och UV
## Läsa in och tillämpa material
Öppna avsnittet **Material** på fliken Yta. **Läs in mapp** lägger till en mapp med bilder och **Läs in .wad** lägger till ett texturpaket; båda visas under **Samlingar**, där krysset tar bort en. Skriv i **Filtrera material** för att begränsa listan. Klicka på ett material för att göra det aktuellt, och **Applicera på markering** lägger det sedan på varje markerad yta. **Markera** erbjuder Markera brushar med det och Markera ytor med det. **Ersätt** byter ett material mot ett annat i hela kartan eller bara i markeringen.

Verktyget **Måla** (`K`) lägger det aktuella materialet på ytan under pekaren: ett klick målar en yta, ett dubbelklick målar hela brushen, och `Shift`-klick hämtar ytans material som aktuellt material, som en pipett.

## UV-redigeraren
Avsnittet **UV** på fliken Yta visar den markerade ytans material och kontur. Muskontroller på arbetsytan:
- Hjulet zoomar runt pekaren. Dra med höger knapp eller mittknappen för att panorera. Dubbelklicka med höger knapp för att rama in ytan igen; när du markerar en annan yta ramas den in automatiskt.
- Dragning med vänster knapp flyttar materialet medan ytans kontur ligger kvar. Ringen roterar mappningen, hörnhandtagen skalar den och mitthandtaget anger origo för dessa ändringar.
- `Alt`-dragning med vänster knapp på en plattkant skjuvar, liksom `Shift`-dragning med mittknappen.
- Varje dragning är ett ångringssteg, och `Esc` avbryter en pågående dragning.

Under arbetsytan finns **Förskjutning**, **Skala**, **Vinkel** och, när spelet använder dem, **Innehållsflaggor**, **Ytflaggor** och **Ytvärde**. Knapparna är **Återställ**, **Återställ till världen**, **Anpassa**, **Vänd H**, **Vänd V** och de två 90°-vridningarna. **Justera** vrider materialet längs en av ytans kanter (tryck igen för nästa kant, `Shift` för föregående). De fyra justeringsknapparna skjuter materialet mot en kant. **Anpassa U** och **Anpassa V** skalar så att ett helt antal kopior täcker ytan; tryck igen för nästa antal, och håll ned `Ctrl` för att behålla ett trimark indelning.

## UV-tangenter
När ytor är markerade flyttar piltangenterna materialet, `[` och `]` vrider det 15°, `,` och `.` halverar eller fördubblar dess skala, `Ctrl+F` och `Ctrl+Alt+F` vänder det, `Shift+R` återställer det, `Alt+Shift+R` återställer det till världsaxlarna och `Ctrl+Shift+F` anpassar det till varje yta.

**Lås** i verktygsfältet behåller mappningen medan brushar flyttas, och **UV** (`U`) behåller den medan brushar vrids.

## Verktygsmaterial
- skip: tar bort den markerade ytan från motorexporter.
- clip: skapar kollision utan synlig geometri.
- origin: markerar pivotpunkten för en brush-entitet.

Namnen kan ändras på kortet **Verktygsmaterial** i exportfönstret. Använd skip för ytor som du vet är dolda.

## Utelämna ytor vid export
Markera en yta och slå på **Utelämna ytan vid export** i avsnittet UV. Orange skraffering markerar den i UV-förhandsvisningen. Dess material och färg förblir sparade, och brushen förblir sluten och redigerbar. Exporterade renderingsmesher utelämnar ytan medan kollisionen behålls. Om en fyrhörning av en kub utelämnas blir det två renderingstrianglar. Detta gör inte ytor dubbelsidiga och slår inte ihop mesher.

**Utelämna täckta ytor** markerar ytor som helt täcks av statiska brushar, inklusive identiska ytor som rör vid varandra och inneslutna brushar. Det fungerar på de markerade brusharna, eller på all synlig olåst världsgeometri när ingenting är markerat. Brush-entiteter, genomskinliga material och brushar som redan har utelämnade ytor hoppas över. Delvis täckta ytor delas inte. En enda ångring återställer allt. Visa → Visa utelämnade ytor döljer eller visar sådana ytor i editorns vyer; markering använder fortfarande hela brushen.

# Färg och gradienter
Avsnittet **Färg** på fliken Yta målar ytor med enfärgade fält eller gradienter ovanpå deras material. Det är användbart för att skissa upp en bana och för stiliserad grafik. Målade ytor behåller sina riktiga färger i vyerna; markeringen visas med sin kontur.

## Palett och blandfärg
Klicka på en av de tolv färgrutorna i paletten för att måla de markerade ytorna, eller hela markerade brushar, med den. **Opacitet** anger hur täckande nästa målning blir, i procent. **Blanda** har en färgknapp som öppnar färgväljaren; dra i väljaren så följer markeringen med direkt, eller tryck på **Måla** för att tillämpa blandfärgen som ett steg. **Tillbaka till material** tar bort färgen så att ytan visar sitt material igen. Raden överst i avsnittet anger vad som är markerat och vad det har på sig.

## Färgväljaren
Alla färgknappar i editorn öppnar samma väljare. Dra i kvadraten för att välja mättnad och ljusstyrka, och i remsan bredvid för att välja nyans. Nedanför finns värdena **R**, **G** och **B** och ett skjutreglage för **Opacitet** där färgen har en sådan. Hexfältet tar emot `#RRGGBB` eller `#RRGGBBAA`; det blir rött så länge texten inte är en giltig färg. **Kopiera** lägger hexkoden i urklipp. Klicka utanför väljaren för att stänga den.

## Gradienter
**Gradient** väljer Linjär gradient eller Radiell gradient, och knappen bredvid vänder på färgernas ordning. Varje stopp har en färgknapp och **Position (%)**, och stopp mellan ändarna har en knapp för borttagning. **Lägg till färgstopp** lägger till ett stopp i den största luckan; en gradient kan ha upp till åtta stopp. **Gradientvinkel** anger riktningen för en linjär gradient, med knapparna **+45°** och **−45°**.

Under förhandsvisningen finns en skena med en markör för varje stopp. Klicka på skenan för att lägga till ett stopp, klicka på en markör för att öppna dess väljare, dra en markör för att flytta den och högerklicka på en markör för att ta bort den. De tre färgrutorna under skenan är förinställningar: Varm gradient, Kall gradient och Gråskalegradient. Ändringar i en gradient tillämpas direkt på markeringen.

En gradients riktning spelar roll: orientera den längs en taklutning eller längs ett böjt objekt, inte tvärs över ytor som inte hör ihop. Kontrollera modellen från båda håll efter målningen.

## Ytmaterial
**Ytmaterial** ställer in **Grovhet** (0 är blankt, 100 är matt), **Metallisk** (0 för trä, sten eller plast, 100 för metall) och **Emission** (0 till 8; ytan lyser i sin egen färg, även gradienter, men lyser inte upp objekt i närheten). Ändringar tillämpas direkt på de markerade ytorna eller brusharna. **Använd standardyta** tar bort åsidosättningen. Alla ändringar kan ångras.

Yteffekter syns i den belysta 3D-vyn och i kamerarenderingar. Platt visning och trådramsvisning utelämnar högdagrarna avsiktligt. Bloom kan lägga till ett sken runt självlysande ytor. Godot och glTF/GLB tar emot materialvärdena, och Unity-importören gör om grovhet till släthet. Se `docs/surface-materials.md` för begränsningarna i varje mål.

# Tillgångar, lager och grupper
## Tillgångsbiblioteket
Öppna fliken **Tillgångar**. Lägg till en mapp med `.map`-modeller med **Lägg till mapp**; det inbyggda biblioteket finns alltid där. Skriv i **Filtrera tillgångar** för att söka. Klicka på en modell för att markera den, och dubbelklicka sedan på den, tryck på `Enter` eller klicka på **Infoga** för att lägga till den där pekaren senast var. Du kan också dra en modell till en vy. En tillgångs brushar infogas som en grupp så att modellen flyttas som en helhet.

**Spara markering som tillgång** skriver de markerade brusharna till biblioteket som en `.map`. Bibliotekets tillgångar är vanliga redigerbara brushar, inte importerade mesher.

## Lager
Listan **Lager** på fliken Karta organiserar stora områden. Skriv ett namn och klicka på **Lägg till** för att skapa ett lager, eller tryck på `Enter` för att byta namn på det aktuella. Varje lager kan döljas, låsas eller utelämnas från exporter. **Flytta hit** flyttar markeringen till det aktuella lagret. Snabbmenyn i vyerna erbjuder också Flytta till lager, Gör till aktivt lager, Dölj lager och Isolera lager.

## Grupper och länkade grupper
`Ctrl+G` grupperar markeringen och `Ctrl+Shift+G` delar upp den. Avsnittet **Grupper** på fliken Karta listar grupper och erbjuder **Öppna**, **Byt namn**, **Slå ihop**, **Dela upp grupp**, **Ta ut** och **Extrahera**. En länkad grupp är en uppsättning kopior som delar redigeringar: `Ctrl+Shift+D` skapar en länkad dubblett, Geometri → Synka länkade grupper uppdaterar kopiorna och **Avlänka** gör en kopia fristående. På fliken Entitet kan en egenskap behållas som en kopias egen när länkade grupper synkas.

## En gemensam skala
Standardskalan för export är 32 kartenheter per meter. Den inbyggda startmarkören för spelaren är 64 enheter hög, en referens på 2 meter. Jämför dörröppningar, fordon, möbler och växter med den. Ändra exportskalan bara avsiktligt och håll relaterade tillgångar konsekventa.

# Entiteter och spellogik
Vanliga entitetsdefinitioner är inbyggda och alltid tillgängliga: ljus, startpunkter för spelare, dörrar, knappar, plattformar, utlösare, statiska rekvisita, ljud och vägpunkter. Om du läser in ett projekts FGD med **Entitetsdefinitioner** på fliken Entitet ersätts klasser med samma namn medan övriga inbyggda behålls.

## Placera entiteter
I bläddraren **Entiteter** på fliken Entitet skriver du i **Filtrera klasser** och väljer en klass. För en punktklass, till exempel ett ljus, klickar du på **Placera** eller dubbelklickar på klassen; `Insert` placerar en i markeringens mitt. För en brushklass, till exempel en dörr eller en utlösare, markerar du brusharna och klickar på **Skapa**. Klasser kan också dras till en vy.

## Egenskaper
Egenskapslistan redigerar den markerade entiteten. Entiteter av samma klass redigeras tillsammans. **Lägg till egenskap** lägger till en nyckel som definitionen kanske inte känner till; **Standardvärden** kan återställa befintliga egenskaper till deras standardvärden eller lägga till de saknade. Egenskaper som klassen definierar men som entiteten saknar visas nedtonade; när du skriver ett värde läggs de till. Använd targetname och target för att länka entiteter.

## Vad en export kan göra
Godots direktiv skapar inbyggda noder, kollisionskroppar och utlösarområden. Unity och Unreal stöder inbyggda vanliga ljus, och Unreal skapar även PlayerStart-aktörer. Övriga data bevaras så långt målets importör tillåter.

En dörrentitet ger inte i sig något spelbeteende. Rörelse, reaktioner på utlösare, spelarkontroller och projektskript kräver fortfarande arbete i motorn. Defold exporterar entitetsegenskaper och kollisionsobjekt; Flax placerar modeller och generiska entiteter. Kontrollera resultatet i din motor innan du bygger spellogik ovanpå det.

# Animering
Heron Brush kan förhandsgranska enkla rörelser och redigera nyckelbildsanimeringar av brush-entiteter, och sedan exportera dem till flera motorer.

## Förhandsgranska animeringar
Slå på **Förhandsgranska animeringar** i menyn Visa, menyn Animering, menyn Rutnätsinställningar eller avsnittet Visa på fliken Karta. Den spelar upp rotationen i func_rotate och fram-och-tillbaka-rörelsen i func_shuttle när start_on är aktiverat, med entitetens egenskaper för axel, hastighet, riktning, sträcka och väntetid, och den spelar upp nyckelbildsklipp som är inställda på Spela upp vid start. Tillgångarna fläkt och väderkvarn innehåller redan roterande delar.

När förhandsvisningen stängs av återställs den ursprungliga ställningen. Ett klick i en vy stoppar uppspelningen innan något markeras eller redigeras; kameranavigeringen fungerar fortfarande. Förhandsvisningen ändrar inte kartan, ångringshistoriken eller exporterna, och den kör ingen fysik eller spellogik.

## Nyckelbildsredigeraren
Öppna den med Animering → Nyckelbildsanimering eller från menyn Rutnätsinställningar. Den dockas under vyerna, eller i 3D-fönstret medan det är separat. Fönsterikonen i dess rubrik flyttar den till ett eget fönster; den behåller bildrutan och ångringshistoriken. **Redigera animering** växlar vyerna till den animerade ställningen och **Redigera karta** går tillbaka till vilogeometrin. I animeringsläget visar varje vy **Redigera animering** och bildrutans nummer längst ned.

Markera en brush-entitet, eller en brush eller brushgrupp, för att animera den. En platt brushgrupp delar en animering och en pivotpunkt; grupper som innehåller flera entiteter behöver separata animeringar.

## Klipp
Väljaren **Animering** växlar mellan namngivna klipp. Skriv ett namn i **Animeringsnamn** och klicka på **Spara som** för att kopiera det aktuella klippet under det namnet; **Ta bort** tar bort ett klipp. **Förflytta** skapar ett `translate`-klipp utan rotation. **Dörranimationer** skapar mjuka klipp `open` och `close` med en vridning på 90 grader kring den aktuella pivotpunkten; flytta först pivotpunkten till gångjärnet. **Spela upp vid start** gör att klippet spelas upp när banan startar; dörrförinställningen stänger av det.

## Transport och uppspelning
Transportknapparna är **Start**, **Föregående nyckel**, **Uppspelning**, **Nästa nyckel** och **Slut**. **Bildruta** visar och ställer in den aktuella bildrutan, och **Varaktighet (s)** anger klippets längd. Läget är **En gång**, **Slinga** eller **Pingpong**.

## Tidslinjen
Tidslinjen har en linjal med spelhuvudet och ett spår för **Position** (X, Y och Z får nycklar tillsammans) och för varje rotationsvinkel, RX, RY och RZ. Klicka eller dra i linjalen för att flytta spelhuvudet och förhandsgranska den ställningen i alla vyer. Dra med mittknappen för att panorera tidslinjen. Dra nycklar för att flytta dem i tiden; `Shift` lägger till i markeringen; dubbelklicka på ett spår för att infoga en nyckel; `Ctrl`+hjulet zoomar. **Kurvor** visar värdekurvorna i stället för spåren.

Knapparna ovanför är **Lägg till nyckel**, **Ta bort**, **Kopiera**, **Klistra in** och **Duplicera**. **Fäst mot bildrutor** håller nycklar på hela bildrutor och **Anpassa** visar hela klippet. **Tajma om nycklar** öppnar **Tidsskala**, **Förskjutning (bildrutor)**, **Skala från spelhuvudet**, **Förläng varaktighet** och **Tajma om markering**.

Nyckelinspektören bredvid tidslinjen redigerar den markerade nyckeln: **Bildruta**, **Värde** (eller de tre koordinaterna för en positionsnyckel), interpolationen **Linjär**, **Konstant** eller **Bézier**, **Ingående lutning** och **Utgående lutning**, samt **Jämna tangenter**.

## Tangenter i redigeraren
Medan tidslinjen har tangentbordet: `Space` spelar upp eller pausar, vänster- och högerpilen stegar en bildruta, `Ctrl+A` markerar alla nycklar, `Ctrl+C`, `Ctrl+X` och `Ctrl+V` kopierar, klipper ut och klistrar in nycklar, `Ctrl+D` duplicerar dem, `Delete` tar bort dem, `Ctrl+Z` ångrar och `Esc` avbryter en dragning. Utanför tidslinjen lämnar `Esc` animeringsläget. Ångra lämnar zoomen, markeringen och spelhuvudet där de var.

## Ställningar i vyerna
Raden under tidslinjen har **Lökskalsvisning** med ett antal och **Avstånd**, som visar närliggande ställningar som spöken; koordinaterna för **Pivotpunkt**; **Automatisk nyckel**; och ställningsverktygen **Förflytta**, **Rotera** och **Pivotpunkt**.

- **Lägg till nyckel** slår på Automatisk nyckel och pausar uppspelningen. Med Automatisk nyckel påslaget och **Förflytta** valt drar du en färgad axel på den markerade entitetens handtag i 3D-vyn för att sätta en nyckel för dess position vid den aktuella bildrutan.
- I en 2D-vy drar du den markerade brushen eller en medlem i dess grupp för att sätta nycklar för förflyttning längs de två synliga axlarna, med fästning mot rutnätet. Automatisk nyckel behöver inte vara påslaget för detta.
- Med **Rotera** drar du brushen i en 2D-vy runt dess orange pivotmarkör. Ovanifrån sätter nycklar för RZ, Framifrån för RY och Sida för RX, fästa mot rutnätets vinklar.
- Med **Pivotpunkt** drar du det gula rotationscentrumet i valfri 2D-vy. Det fästs mot rutnätet och behåller sitt djup. Nycklar och vilogeometri förblir oförändrade.

Den cyanfärgade banan i 2D och 3D visar pivotpunktens rörelse. Klicka på en punkt på banan för att markera dess tid och nycklar, eller dra den för att ändra positionen; med Automatisk nyckel påslaget kan du också dra banpunkter i 3D-vyn. Etiketter visar bildrutenummer och streckade segment markerar hopp. `Esc` avbryter alla dessa dragningar, och en enda ångring återställer en hel dragning.

Nya animeringar börjar i viloläge med nycklar vid tiden noll. Skapa rörelse genom att ändra ställningen vid en annan bildruta. För en skarp L-formad bana sätter du nycklar för start, hörn och slut, och använder Linjär med Pingpong.

## Koppla knappar till animeringar
Markera en func_button, trigger_multiple eller trigger_once. Fliken Entitet visar **Animeringsutlösare** ovanför egenskaperna. Välj ett mål i listan, eller klicka på **Välj mål** och sedan på ett animerat objekt i valfri vy; `Esc` avbryter valet. Saknade mål eller klipp rapporteras under kontrollerna. Entitetslänkar visar pilar till sina mål när länkvisning är påslagen.

**Åtgärd** väljer vad aktiveringen gör: **Spela upp** startar ett namngivet klipp eller den markerade animeringen; **Växla** alternerar mellan `open` och `close` när de finns, annars startar den om den markerade animeringen; **Öppna** och **Stäng** startar de klippen; **Pausa** håller ställningen, **Återuppta** fortsätter och **Stoppa** återställer den ursprungliga ställningen. **Bara en gång** åsidosätter klassens standardvärde. **Upprepningsfördröjning (s)** anger kortaste tid mellan aktiveringar; den fördröjer inte den första.

**Testa utlösare** kör åtgärden i förhandsvisningen utan att redigera kartan eller lägga till ångringssteg. **Återställ förhandsvisning** återställer de ursprungliga ställningarna och rensar engångs- och nedkylningstillstånd. Detta simulerar inte spelarkollisioner eller spelskript.

## Animering i exporter
Godot exporterar inbyggda AnimationPlayer- och AnimationLibrary-resurser, inklusive kurvor och rörlig kollision. En CharacterBody3D som går in i en knapps område aktiverar dess mål. Ändringar som görs i dessa klipp i Godot kommer inte tillbaka till kartan och ersätts vid nästa export.

Raylib-exporten lägger till `<map>_animation.h` med en C-spelare och `<map>_preview.c` med en förhandsvisning med fri kamera; följ `<map>_raylib.md` för att bygga den. I förhandsvisningen pausar `SPACE`, `R` startar om standardklippen och `N` växlar mellan namngivna klipp. Defold bäddar in uppspelningen i entitetens skript med dokumenterade `heron_*_animation`-meddelanden. Unitys Build Level lägger till `HeronBrushAnimation` och en kinematisk kropp. Bevy-exporten innehåller `heron_animation.rs` och instruktioner. Godot, Unity, Defold, Bevy och raylib behåller utlösarlänkarna, engångsflaggan och upprepningsfördröjningen. Ditt spel måste fortfarande registrera spelaren och koppla sina egna knappar till varje körmiljös API.

## Exemplet Motion lab
Öppna `motion_lab` bland Exempel på välkomstskärmen. Det har en plattform med L-bana, en hiss, en roterande plattform och en dörr på gångjärn med knappar. Öppna `godot/project.godot` som ligger bredvid i Godot och tryck på `F5`. WASD och musen flyttar spelaren, Space hoppar, 1 till 4 besöker stationerna och R startar om den aktuella stationen. Kliv på en färgad platta för att aktivera dess mål. Den medföljande README-filen förklarar hur man exporterar på nytt.

# Exportera till en motor
Arkiv → Exportera öppnar exportfönstret. Motorerna listas längs vänsterkanten med sina logotyper: Godot, Defold, three.js, Bevy, raylib, Unity, Unreal Engine, Flax Engine, Cocos Creator, Wavefront .obj, glTF .gltf, GLB .glb och sist `.map`. Sidan till höger beskriver vad målet får. **Skriv** skriver det, **Kontrollera** bygger banan utan att skriva den och listar problemen, och **Stäng** stänger fönstret.

Export är skild från sparande: `.map` förblir din redigerbara källa. Varje kontroll sparar motorns inställningar så snart den ändras, så varje motor behåller sina egna svar och fönstret öppnas som du lämnade det.

## Gemensamma alternativ
- **Skala**: **Enheter per meter**, 32 som standard.
- **Vad som ska skrivas**: **Kollisionsgeometri**.
- **Optimeringar**: **Gruppera statiska mesher i celler om 1024 enheter**, **Släpp ytor begravda inuti geometrin**, **Slå ihop ytor som delar plan** och **Slå ihop kollisionsformer**.
- **Projektsökvägar**: var banan och dess resurser hamnar i projektet.
- **Verktygsmaterial**: namnen på materialen clip, skip och origin.
- **Mer**: motorspecifika växlar som **Namnge noder efter**, **Återskapa lager och grupper som föräldranoder**, **Vänd triangelns varvriktning**, vändning av texturkoordinater och en andra UV-uppsättning för bakat ljus med **Ljuskarte-texel**.
- **Hålla jämna steg med kartan**: **Skriv igen varje gång kartan sparas**. Det blir tillgängligt efter att motorn har exporterats en gång, eftersom den exporten anger var det ska skrivas.

Statiska celler tilldelar världsbrushar efter deras mittpunkter och kombinerar dem per material inom varje cell om 1024 enheter, så att motorer kan gallra bort mindre delar av kartan. Kollisionen behålls och spelentiteter hålls separata. Det kan ge fler ritanrop än en enda världsmesh, så det är avstängt som standard. Fristående OBJ-, glTF- och GLB-exporter använder inte celler.

## Anteckningar om målen
- Godot: `res://`-sökvägar för texturer och material, utlösarnas fysiklager och lightmap-UV2. Välj en exportmapp inuti Godot-projektet.
- Defold: tillgångarnas resurskatalog, modellmaterialet och textursamplern, som måste finnas i det valda materialet.
- Unity: OBJ-mesher och en importör för editorn; exportera under projektets Assets-mapp.
- Unreal Engine: ett genererat importskript och en innehållssökväg, normalt /Game/<level>.
- Flax Engine: en innehållssökväg för modeller, relativ till projektets Content-mapp.
- three.js, Bevy, raylib och Cocos Creator: scener eller kod för dessa körmiljöer, med instruktioner bredvid exporten.
- glTF / GLB: portabel geometri för kompatibla verktyg; GLB är den binära formen. OBJ: geometri och material för modelleringsverktyg.
- `.map`: **Skriv som** väljer Valve 220, Standard, Quake 2 eller Quake 2 (Valve). Fönstret anger vad den valda dialekten skulle förlora.

Läs varningarna efter skrivningen, till exempel om saknade texturer. **Vad kontrollen hittade** listar dem i fönstret.

# Livelänkar: Godot, Unity, Defold och Flax
En livelänk skickar osparade ändringar från Heron Brush till en motors editor medan du arbetar. Den är enkelriktad: ändringar som görs i motorn kommer inte tillbaka. En livelänk kan köras per kartfönster. Stoppa den innan du byter motor, och starta om den för att tillämpa ändrade exportalternativ. Kontrollerna finns i exportfönstret, på motorns sida, på kortet **Livelänk med**.

Indikatorn i namnlisten visar länkens tillstånd; klicka på den för att stoppa länken. Om kartan stängs eller ersätts upphör publiceringen. Efter en misslyckad import åtgärdar du problemet och startar länken igen.

## Godot
1. Exportera en gång till ett Godot-projekt.
2. Klicka på **Installera Godot-tillägg**.
3. Aktivera Heron Brush Live i Godots Project Settings → Plugins.
4. Klicka på **Starta livelänk** och koppla kartan från Godot-dockan till en öppen 3D-scen.

Tillägget ersätter bara sin genererade gren och behåller bilagans placering och övriga noder. Spara Godot-scenen själv, och håll dina egna skript och noder utanför den genererade grenen.

## Unity
1. Exportera en gång under Unity-projektets Assets-mapp.
2. Klicka på **Installera Unity-tillägg** och låt Unity kompilera det.
3. Öppna Window → Heron Brush → Live Link i Unity.
4. Starta länken och koppla kartan till den aktiva Unity-scenen.

Håll Unitys livefönster öppet. Uppdateringar pausas i Play Mode och Prefab Mode. Bara barnobjektet Generated under kartobjektet ersätts. Redigera inte de genererade filerna.

## Defold
1. Exportera en gång till ditt Defold-projekt och håll projektet öppet i Defold.
2. Klicka på **Starta livelänk**. Inget tillägg behövs.
3. Öppna heron_brush_live/preview.collection i Defold, eller referera till den från din egen scen.

Länken skriver till en separat genererad mapp och lämnar den slutliga exporten och game.project orörda. **Ladda om spelet som körs** uppdaterar också ett spel som startats från Defold; att lägga till objekt eller ändra kollision kan kräva att banan startas om. Misslyckade anslutningar försöks igen automatiskt.

## Flax
Exportera till projektets Content-mapp och klicka på **Installera Flax-tillägg**. Låt Flax kompilera det, starta länken och välj sedan Tools → Heron Brush: attach live map i den öppna Flax-scenen. Den genererade roten behåller sin placering och andra objekt lämnas orörda. Spara scenen själv. Uppdateringar pausas i Play Mode. Tools → Heron Brush: stop live map pausar mottagningen.

# Gemensamma livesessioner
Samarbete → Livesession öppnar ett fönster för att vara värd för eller ansluta till en annan Heron Brush-användare. Det delar kartredigeringen mellan personer; det är skilt från en livelänk till en motor.

## Vara värd och gå med
Ange **Ditt namn**. För att vara värd väljer du **Var värd för en session**, klickar på **Starta session** och skickar uppgifterna från **Kopiera anslutningsuppgifter** till dina gäster. För att gå med väljer du **Gå med i en session**, klistrar in uppgifterna och klickar på **Gå med**. När du går med ersätts den öppna kartan med värdens karta, så spara ditt arbete först.

Standardversionen fungerar i det lokala nätverket, och **Port i lokalt nätverk** anger porten. Sessioner via internet (**Anslut via internet**) kräver en version med den valfria nätverksfunktionen. Deltagarna ser varandras kameramarkörer och namn i 3D-vyn.

Om du stänger fönstret fortsätter sessionen; indikatorn i namnlisten visar dess tillstånd. **Stoppa session** eller **Lämna session** avslutar den. När två personer redigerar samma objekt avgör värdens ordning den slutliga versionen, och även ångra är en delad redigering. Arbeta i separata områden där det går, och spara kontrollpunkter.

# 3D-visning och en andra skärm
## Visningslägen
Avsnittet **Visa** på fliken Karta växlar vad vyerna ritar: Brushar, Punktentiteter, Entitetsmodeller, Klassnamn för entiteter, Gruppkonturer, Patchar, Material, Visa utelämnade ytor, Förhandsgranska animeringar, Trådram, Kanter och Konturer för brush-entiteter. Trådram ritar triangelkanter, inklusive diagonaler och kanter bakom ytor, utan material. Det är en tillfällig felsökningsvy som inte påverkar exporter eller renderingar.

Belysning, skuggor, ambient ocklusion, reflektioner, bloom och kantutjämning för 3D-vyn finns i Inställningar → Renderare. Det är förhandsvisningsinställningar; färdiga bilder använder renderingskamerans egna inställningar.

## Ett separat 3D-fönster
Visa → 3D-vy i separat fönster flyttar 3D-vyn till ett eget fönster, som du kan placera på en annan skärm. Det delar kamera, dokument och verktyg, och de platta vyerna tar platsen i huvudfönstret. Dess meny erbjuder Material, Solid vy och Trådram; en undermeny Visa med Visa utelämnade ytor, Förhandsgranska animeringar, Kanter, Gruppkonturer, Punktentiteter, Entitetsmodeller och Konturer för brush-entiteter; Fokusera på markering; Framifrån, Sida och Ovanifrån, som vrider kameran runt markeringen (eller en punkt framför kameran); Placera kamera från 3D-vyn; Helskärm (`F11`); Rendera bild; och Återgå till huvudfönstret. När fönstret stängs dockas vyn igen.

## Procedurell himmel
Den procedurella himlen är påslagen som standard i 3D-vyn och kan stängas av i Inställningar → Renderare → Ljus. Renderingskameror har en egen växel **Procedurell himmel**, som sparas med kartan. Färgerna för himmel, horisont och mark kommer från miljöinställningarna, och solen följer scenens belysning. Himlen har ingen geometri och exporteras inte.

# Renderingskameror och PNG-bilder
Arkiv → Rendera bild öppnar renderingsfönstret: kameran och dess inställningar till vänster och en direktförhandsvisning till höger som ritas om när du ändrar dem. Kameror sparas i `.map` i lagret Render cameras, som bara finns i editorn och utelämnas från motorexporter.

## Placera kameran
Komponera bilden i 3D-vyn och tryck på `Ctrl+Alt+0` (eller `Ctrl+Alt+Num0`), eller klicka på **Placera kamera från 3D-vyn**. Om ingen kamera är markerad uppdateras den första sparade; om ingen finns skapas en. Detta kan ångras och behåller inställningarna för utdata och belysning. **Markera kameran i kartan**, **Läs in kamera från kartan** och **Flytta 3D-vyn till kameran** fungerar åt andra hållet. **Kameraram i 3D-vyn** visar utdatans gräns, och **Styr kameran från 3D-vyn** låter vanlig navigering flytta den sparade kameran.

## Inställningar
- **Kameraposition**: X, Y, Z, gir och tipp.
- **Utdata och objektiv**: **Förinställningar** (Full HD, Kvadrat, Stående), bredd, höjd, sampel, **Vertikalt synfält** och **Bortre klipplan**.
- **Belysning**: Dagsljus, Solnedgång eller Mulet, eller ställ in exponering, solen, färgerna för himmel, horisont och mark, **Procedurell himmel**, **Skuggor**, **Ambient ocklusion** och **Bloom**.
- **Skärpedjup**: **Aktivera skärpedjup**, fokusavstånd, skärpeområde och oskärperadie. **Fokusera på markering** fokuserar på mitten av ett markerat objekt framför kameran.

**Spara kamera och inställningar** lagrar formuläret i kartan; spara kartan för att behålla det på disk.

## Rendering
**Rendera PNG** frågar var filen ska sparas och startar en progressiv rendering, som fylls i sampel för sampel i förhandsvisningen. Fler sampel ser bättre ut men tar längre tid. **Avbryt rendering** stoppar utan att skriva någon fil. Renderaren använder editorns rasterpipeline, inte strålspårning. PNG-filen har en ogenomskinlig bakgrund, och animeringar renderas inte. Mycket stora bilder kan kortvarigt få editorn att stanna upp.

# Inställningar
Arkiv → Inställningar (`Ctrl+,`) öppnar inställningarna. Ändringar gäller direkt och sparas. Sidofältet listar Översikt, Spel, Visa, Färger, Mus, Tangentbord och Renderare. Sökfältet överst hittar en inställning efter namn eller beskrivning på valfri sida. Varje ändrad inställning har en egen knapp som återställer dess standardvärde, och **Återställ standardvärden** återställer den aktuella sidan.

## Sidorna
- **Översikt**: de vanliga inställningarna samlade: Utseende (Panelopacitet (%), Färgschema, Layout, Språk), Redigering, Visa och Handbok.
- **Spel**: spelet, var dess filer finns och ditt namn för samarbete. Ett spel anger vilket kartformat som ska skrivas, vilka entitetsdefinitioner som ska läsas in och var dess material finns. Spelkonfigurationer läses från mappen `games` i konfigurationsmappen (`~/.config/heron-brush/games` på Linux och macOS, `%LOCALAPPDATA%\heron-brush\games` på Windows) och från mappen `games` bredvid programmet, där paketen lägger den medföljande konfigurationen.
- **Visa**: Layout, Färgschema, Språk, världsaxlar, rutnätets opacitet, cellstorlek för läckkontroll och mappningslåsen.
- **Färger**: färgerna för vyer, geometri och handtag.
- **Mus**: hastigheter och inverteringar för att se, panorera och flytta, flyghastighet, synfält, Länka 2D-vyer och handtagens storlek.
- **Tangentbord**: tangentbordsmappningen, Heron Brush eller Blender.
- **Renderare**: realistisk skuggning, markeringskontur, ljus, skuggor, ambient ocklusion, reflektioner, bloom och ytor.

## Språk
**Språk** erbjuder Följ skrivbordet och alla översatta språk. Gränssnittet ändras direkt. Handboken öppnas på gränssnittets språk när det finns en översättning av den, och annars på engelska.

# Tillägg och MCP
## Tillägg
Tillägg utökar editorn med Rhai-skript. Arkiv → Tillägg öppnar fönstret Tillägg. **Installera ett tillägg** installerar en `.rhai`-fil och **Installera en tilläggsmapp** installerar en mapp som innehåller `main.rhai`; valfri metadata i `plugin.toml` beskriver tillägget och dess inställningar. Tilläggen grupperas under **Aktiva tillägg** och **Inaktiva tillägg**, och **Aktivera tillägg** slår på eller av ett tillägg; detta består efter en omstart. **Läs in tillägg igen** läser skripten från disk på nytt och återställer deras panelvärden.

Aktiva tillägg visas också i avsnittet **Tillägg** på fliken Karta med sina inställningar, åtgärdsknappar (**Kör**) och utdata. En åtgärd verkar på markeringen, eller på allt när ingenting är markerat, och är ett ångringssteg. Den medföljande markeringsinspektören visar antal samt åtgärder för att flytta upp och ned.

## MCP
Samarbete → Tillhandahåll för en agent (MCP) låter en AI-agent eller ett annat verktyg styra den öppna editorn via Model Context Protocol. Fönstret visar den lokala adressen med **Kopiera**, och **Stoppa server** avslutar den; medan den körs visar namnlisten **MCP aktivt**. Kommandoradsalternativet `--mcp` kör samma gränssnitt utan fönster. Klienter kan upptäcka verktyg för geometri, tillgångar, egenskaper, animering, export och sessioner. MCP och editorns tillägg är olika gränssnitt.

# Kommandoraden
Programmet heter `heron-brush`. Utan alternativ öppnar det välkomstskärmen.

## Öppna filer
`heron-brush map.map [entities.fgd] [materials/]` öppnar en karta, valfritt med entitetsdefinitioner och en materialmapp. Ett tomt argument hoppar över ett av dem, till exempel `heron-brush map.map "" materials/`. Om en editor redan körs öppnas kartan i den; `--new-instance` öppnar i stället ett separat fönster.

## Kommandon
- `--help` eller `-h`: skriv ut användningen. `--version` eller `-V`: skriv ut versionen.
- `--mcp`: tillhandahåll MCP via standard in och standard ut, utan fönster.
- `--merge-map <base> <ours> <theirs> [<output>]`: en trevägssammanslagning av `.map`-filer för git. Utdata hamnar som standard i ours. Konflikter skrivs ut och ger en slutkod skild från noll.
- `--obj-to-map <in.obj> <out.map> [size] [thickness] [snap] [material]`: gör om en mesh till brushar. Size är längden på den längsta sidan i kartenheter (standard 256), snap är ett rutnät för hörnen (0 för inget), och orden `solid` eller `layered` bygger rutnätslådor eller konvexa skivor i stället för tunna kilar.
- `--render-preview <in.map> <out.png> [size]`: skriv en PNG-miniatyr av en karta, 512 pixlar som standard.
- `--run-script <script.rhai> <in.map> [out.map] [key=value ...]`: kör ett tilläggsskript på en karta med allt markerat och skriv resultatet (som standard tillbaka till indatafilen). `key=value` besvarar tilläggets inställningar.

# Felsökning
## När något ser fel ut
- Saknat material: kontrollera materialmappen eller paketet och exportens projektsökvägar.
- Fel modellstorlek: jämför med spelarmarkören på 64 enheter och exportskalan.
- Fel gradientriktning: kontrollera gradientvinkeln och ytans orientering.
- Tom motorscen: läs exportvarningarna och följ målets importinstruktioner.
- Livelänken väntar: kontrollera att tillägget är installerat, att scenen är kopplad och att uppdateringarna inte är pausade.
- Bara linjer, inga ytor: stäng av Trådram i avsnittet Visa på fliken Karta, eller välj Material eller Solid vy i 3D-fönstrets meny.
- Objekt saknas i vyerna: `Ctrl+Shift+I` visar dolda objekt; kontrollera lagren och View filters.
- En tangent gör ingenting: klicka först i vyn så att den har tangentbordet, och kontrollera tangentbordsmappningen i Inställningar → Tangentbord.
- En redigering misslyckades: ångra, förenkla geometrin och använd **Kontrollera karta** och **Sök efter läckor** i avsnittet **Problem** på fliken Karta.
- En karta går inte att öppna från filhanteraren: den kan ha lämnats över till en editor som redan körs; leta efter det fönstret.

## Inställningar och säkerhetskopior
Inställningar, senaste kartor, stjärnor, exportinställningar och tillägg finns i konfigurationsmappen: `~/.config/heron-brush` på Linux och macOS, och `%LOCALAPPDATA%\heron-brush` på Windows. Säkerhetskopior från automatisk sparning finns i mappen `autosave` bredvid varje karta.

## Rapportera ett fel
Ange editorns version (Hjälp → Om), ditt operativsystem, en liten karta som visar problemet och de exakta stegen. Utdatarutan (`Ctrl+4`) visar ofta vad som gick fel. Spara en kopia innan du experimenterar.

## Använda den här handboken
Välj ett kapitel i innehållsförteckningen till vänster. `Ctrl+F` visar innehållsförteckningen och flyttar till sökfältet; `Esc` rensar sökningen, eller stänger handboken när sökningen är tom. **Innehåll** döljer eller visar kapitellistan och **Textstorlek** ändrar teckenstorleken; båda sparas till nästa gång. Språklistan överst växlar mellan de tillgängliga översättningarna.
