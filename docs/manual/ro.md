# Primii pași
Heron Brush este un editor de niveluri bazat pe brush-uri. Construiești niveluri din brush-uri convexe, le colorezi și le texturezi fețele, plasezi entități, animezi uși și platforme și exporți rezultatul către un motor de joc. Are și asset-uri încorporate, forme parametrice, legături live cu editoarele motoarelor, sesiuni de editare comune și randare de imagini statice. Acest manual vine împreună cu aplicația și funcționează offline.

Pe macOS, `Cmd` funcționează oriunde manualul spune `Ctrl`, iar Option este Alt.

## Ecranul de bun venit
Când Heron Brush pornește fără o hartă, se deschide ecranul de bun venit. În stânga sunt **Continuă …** (ultima hartă pe care ai avut-o deschisă), **Hartă nouă** și **Răsfoiește** pentru deschiderea unui fișier `.map` existent, plus **Joc**, **Manual de utilizare** și **Preferințe**. În dreapta este lista hărților recente cu miniaturi, urmată de **Exemple**. Steaua (**Păstrează deasupra**) fixează o hartă în partea de sus a listei; coșul (**Mută la coșul de gunoi**) mută fișierul în coșul de gunoi al sistemului, de unde poate fi restaurat. Tastele săgeți te mută prin ecran, iar Enter deschide elementul selectat. Fișier → Bun venit readuce oricând acest ecran.

## Prima ta hartă
1. Alege **Hartă nouă** pe ecranul de bun venit sau Fișier → Hartă nouă (`Ctrl+N`).
2. Alege unealta Brush (`B`) din bara de unelte și trage într-o vizualizare 2D pentru a desena forma selectată. Verifică-i adâncimea într-o altă vizualizare 2D.
3. Selectează brush-ul și mută-l prin tragere. Folosește fila **Față** pentru a-i da un material sau o culoare.
4. Salvează harta sursă cu `Ctrl+S`. Când ești gata să o folosești într-un joc, deschide Fișier → Exportă și scrie o scenă pentru motorul tău.

## Fișiere și salvare
Fișierul `.map` este sursa ta editabilă. Salvează des: exportul, legăturile live și randarea nu salvează niciodată harta în locul tău. Fișier → Salvează ca (`Ctrl+Shift+S`) adaugă extensia `.map` dacă o omiți. Fișier → Revino la harta salvată renunță la modificările nesalvate și reîncarcă fișierul. Dacă închizi o fereastră sau deschizi altă hartă cu modificări nesalvate, Heron Brush te întreabă ce vrei să faci: **Salvează**, **Abandonează** sau **Renunță**.

Cât timp o hartă salvată are modificări nesalvate, la fiecare zece minute se scriu copii de rezervă numerotate într-un dosar `autosave` de lângă hartă. Se păstrează cele mai noi 50 de copii.

Dacă deschizi o hartă în timp ce Heron Brush rulează deja, ea este preluată de fereastra care rulează, iar aceasta trece în față. Folosește `--new-instance` în linia de comandă când vrei o fereastră separată (vezi Linia de comandă).

# Interfața
Fereastra principală are o bară de titlu cu meniurile, o bară de unelte sub ea, vizualizările hărții, inspectorul în dreapta și o bară de stare în partea de jos.

## Bara de titlu și meniurile
Bara de titlu afișează sigla, meniurile, numele hărții la mijloc și butoanele ferestrei. Pe macOS, butoanele obișnuite roșu, galben și verde sunt în stânga. Trage bara de titlu pentru a muta fereastra și fă dublu clic pe ea pentru a o maximiza. Cât timp ceva rulează în fundal, aproape de capătul barei apare un indicator: o legătură live cu un motor, o sesiune live sau **MCP activ**. Fă clic pe un indicator pentru a opri legătura live sau pentru a-i deschide fereastra.

- **Fișier**: Bun venit, Hartă nouă (`Ctrl+N`), Deschide hartă (`Ctrl+O`), Salvează (`Ctrl+S`), Salvează ca (`Ctrl+Shift+S`), Revino la harta salvată, Exportă, Randează imaginea, Pluginuri și Preferințe (`Ctrl+,`).
- **Editare**: Anulează (`Ctrl+Z`), Refă (`Ctrl+Shift+Z`), Decupează, Copiază, Lipește, Duplică (`Ctrl+D`), Șterge, Selectează tot (`Ctrl+A`), Repetă ultimele comenzi (`Ctrl+R`) și Uită comenzile repetabile.
- **Vizualizare**: Vizualizare 3D în fereastră separată, Mergi la, Aliniază la grilă (`Alt+0`), Arată grila (`0`), Previzualizează animațiile și Arată fețele omise.
- **Geometrie**: Inserează text 3D, Îmbinare colț, Convertește fețele în patch-uri, Scoate selecția din entitatea sa și Sincronizează grupurile legate.
- **Animație**: Animație cu cadre cheie, Previzualizează animațiile și Plasează camera din vizualizarea 3D (`Ctrl+Alt+0`).
- **Hartă**: Încarcă, Reîncarcă și Descarcă fișierul de puncte; Încarcă, Reîncarcă și Descarcă fișierul de portaluri. Acestea arată traseele scurgerilor și portalurile scrise de compilatoarele de hărți.
- **Colaborare**: Sesiune live și Servește pentru un agent (MCP).
- **Ajutor**: Manual de utilizare și Despre.

## Bara de unelte
Bara de unelte conține toate uneltele. Primul grup se folosește tot timpul: Selectează, Brush, Tăiere, Vârf, Muchie, Față, Rotește și Scalare. Al doilea grup cuprinde Forfecare, Asamblare, Baleiere, Punct de control, Rotunjire și Vopsea. Ține cursorul peste un buton pentru a-i vedea numele și scurtătura. Un clic pe unealta activă te readuce la Selectează.

După unelte urmează **Oglindește orizontal** și **Oglindește vertical**, apoi două comutatoare: **Blochează** păstrează maparea materialului când un brush se mută, iar **UV** o păstrează când un brush se rotește (`U` îl comută). Urmează setările uneltei active: unealta Brush arată forma și opțiunile ei, iar unealta Rotunjire arată raza, fețele și colțurile, cu **Aplică**. La capătul din dreapta sunt numele uneltei, dimensiunea grilei (**Grilă 16** și așa mai departe; fă clic pentru a alege alta) și meniul **Setări grilă** cu Aliniază la grilă, Arată grila, Previzualizează animațiile, Arată fețele omise și Animație cu cadre cheie.

## Vizualizările hărții
Există patru vizualizări: vizualizarea 3D și trei vizualizări plane, Sus (XY), Frontal (XZ) și Lateral (YZ). Preferințe → Vizualizare → Aranjament alege Un panou, Două panouri, Trei panouri sau Patru panouri. Trage separatoarele pentru a redimensiona panourile. Un clic într-o vizualizare îi dă tastatura. `Space` trece un panou care arată câte o vizualizare la următoarea vizualizare, iar `Ctrl+Space` maximizează vizualizarea care are tastatura și apoi restaurează aranjamentul.

Fă clic dreapta fără să tragi în orice vizualizare pentru a deschide un meniu contextual pentru selecție și pentru ce se află sub cursor. Oferă grupare, straturi, grupuri legate, crearea de entități, mutarea brush-urilor într-o entitate sau din ea, copierea numelui unui material și afișarea lui în browserul de materiale.

## Inspectorul
Inspectorul din dreapta are patru file. Trage de marginea lui stângă pentru a-i schimba lățimea.

- **Hartă** (`Ctrl+1`): Straturi, Grupuri, Pluginuri, Probleme, filtre de vizualizare și Etichete.
- **Entitate** (`Ctrl+2`): browserul de entități, definițiile de entități, proprietățile entității selectate și, pentru butoane și declanșatoare, Declanșator animație.
- **Față** (`Ctrl+3`): UV, Materiale, Culoare, Baleiere, Rotunjire, Patch și Retragere și teșire. Fiecare secțiune poate fi pliată.
- **Asset-uri**: biblioteca de modele din brush-uri reutilizabile.

`Ctrl+5` ascunde sau arată inspectorul, iar `Ctrl+Alt+T` ascunde sau arată bara de unelte, ceea ce lasă mai mult loc vizualizărilor.

## Bara de stare și panoul de ieșire
Bara de stare din partea de jos arată cel mai recent mesaj al editorului. Erorile apar cu roșu. Fă clic pe bară sau apasă `Ctrl+4` pentru a deschide panoul de ieșire cu istoricul complet al mesajelor. Majoritatea comenzilor raportează aici ce au făcut, de exemplu câte brush-uri au fost tăiate.

## Dialoguri și aspect
Dialogurile precum Preferințe, Exportă și Randează imaginea se deschid în ferestre proprii. Preferințe → Vizualizare → Schemă de culori alege Ca în sistem, Luminoasă sau Întunecată. **Opacitate panou (%)** din pagina Prezentare generală face panourile transparente deasupra vizualizărilor. Fereastra își desenează singură butoanele în stilul sistemului tău: nativ pe macOS, în stil Windows 11 pe Windows și în stil GNOME pe Linux.

# Navigare și tastatură
## Deplasarea în vizualizarea 3D
Ține apăsat butonul drept al mouse-ului și trage pentru a te uita în jur. Trage cu butonul din mijloc pentru a deplasa vederea. Rotița te mută înainte și înapoi. `W`, `A`, `S` și `D` zboară înainte, la stânga, înapoi și la dreapta, `Q` urcă, iar `X` coboară; ține `Shift` pentru pași mai mari. Cu **Alt + tragere cu butonul din mijloc mișcă camera** activat în Preferințe → Mouse, Alt împreună cu butonul din mijloc mișcă de-a lungul direcției de privire. Vitezele, inversările și câmpul vizual sunt în Preferințe → Mouse.

## Deplasarea în vizualizările 2D
Trage cu butonul din mijloc pentru a deplasa vederea și folosește rotița pentru a mări sau micșora în jurul cursorului. Deplasarea și mărirea unei vizualizări plane le păstrează aliniate pe celelalte; **Leagă vizualizările 2D** din Preferințe → Mouse controlează acest lucru. `Ctrl+Alt+Z` resetează mărirea, iar `Ctrl+Shift+U` readuce vizualizările plane la origine.

`Ctrl+U` încadrează selecția în fiecare vizualizare. Vizualizare → Mergi la (`Ctrl+Alt+M`) mută camera la coordonatele introduse, selectează ce se află la o anumită linie a fișierului hărții sau mută selecția cu o valoare introdusă.

## Grila și alinierea
`+` și `-` schimbă dimensiunea grilei, iar tastele numerice de la `1` la `9` aleg direct o dimensiune, de la 1 la 256 de unități. `0` sau `#` arată sau ascunde grila, iar `Alt+0` activează sau dezactivează alinierea. `S` aliniază vârfurile brush-urilor selectate la grila curentă, `Ctrl+Shift+V` le aliniază la unități întregi, iar `Ctrl+Alt+Shift+V` la grilă.

## Scurtături uzuale
- `Ctrl+S`: salvează harta. `Ctrl+Shift+S`: salvează ca.
- `Ctrl+Z`: anulează. `Ctrl+Shift+Z` sau `Ctrl+Y`: refă.
- `Ctrl+C`, `Ctrl+X`, `Ctrl+V`: copiază, decupează și lipește. Lipirea pune obiectele la cursor; `Ctrl+Alt+V` le păstrează coordonatele originale. Când sunt selectate fețe, copierea preia materialul și setările UV ale acestora.
- `Ctrl+D`: duplică. `Ctrl+Shift+D`: duplicat legat al unui grup.
- `Delete` sau `Backspace`: șterge selecția.
- `Esc`: anulează tragerea în curs, face un pas înapoi în unealta activă, revine la Selectează și, în final, golește selecția. `Shift+Esc` revine direct la Selectează.
- `Ctrl+R`: repetă ultimele comenzi repetabile. `Ctrl+Shift+R`: le uită.
- `Ctrl+Alt+0` sau `Ctrl+Alt+Num0`: plasează camera de randare în poziția vizualizării 3D.
- `F5`: reîncarcă materialele. `F6`: reîncarcă definițiile de entități.

## Scurtături de selecție
- `Ctrl+A`: selectează tot sau, cât timp este activă o unealtă pentru vârfuri, muchii sau fețe, toate mânerele. `Ctrl+Shift+A`: nu selectează nimic. `Ctrl+Alt+A`: inversează selecția.
- `Ctrl+B`: selectează celelalte obiecte din aceeași entitate sau din același grup.
- `Ctrl+T`: selectează tot ce ating brush-urile selectate. `Ctrl+E`: selectează tot ce se află în interiorul lor. `Ctrl+Shift+E`: selectează tot ce se află în coloana lor de-a lungul axei de vizualizare. Aceste comenzi consumă brush-urile folosite la selecție.
- `Ctrl+I`: izolează selecția. `Ctrl+Alt+I`: o ascunde. `Ctrl+Shift+I`: arată din nou totul.
- `Ctrl+G`: grupează. `Ctrl+Shift+G`: degrupează.

## Scurtături de transformare
- Tastele săgeți și `Page Up` / `Page Down` mută selecția cu un pas de grilă în direcția pe care o indică în vizualizarea activă. Cu `Ctrl` duplică în timpul mutării, iar cu `Alt` rotesc.
- `Alt+X`, `Alt+Y`, `Alt+Z`: rotesc cu un sfert de tură în jurul axei respective; adaugă `Shift` pentru direcția opusă.
- `Ctrl+F`: oglindește peste vizualizare; `Ctrl+Alt+F`: oglindește sus-jos. `Ctrl+Shift+X` și `Ctrl+Shift+Y` oglindesc de-a lungul lui X sau Y.
- `Insert`: plasează o entitate punct în centrul selecției.
- `Tab`: cu un brush selectat, trece ciclic prin Selectează → Vârf → Muchie → Față → Selectează.

## Asocierile de taste Blender
Preferințe → Tastatură → Asocieri de taste oferă Heron Brush (implicit, folosit în tot acest manual) și Blender. Cu Blender: `G` selectează și mută, `R` rotește, `S` scalează, `E` baleiază, `1`, `2` și `3` aleg uneltele pentru vârfuri, muchii și fețe, `K` taie, `B` selectează, `A` selectează tot, iar `Alt+A` renunță la selecție, `X` șterge, `Shift+D` duplică, `H` ascunde, iar `Alt+H` arată totul, și `Ctrl+Shift+Z` reface. Restul este la fel în ambele.

# Brush-uri și unelte de modelare
Brush-urile sunt corpuri convexe definite de plane. Construiește obiectele complexe din mai multe brush-uri simple în loc să încerci să faci un singur brush concav. Păstrează un număr mic de brush-uri acolo unde ajunge o formă simplă.

## Desenarea brush-urilor
Alege unealta Brush (`B`) și trage într-o vizualizare. Bara de unelte arată **Forma de desenat**: Cuboid, Scări, Arcadă, Cilindru, Con, Sferoid (UV), Sferoid (icosaedru) și Inserează text 3D. `Alt+1` până la `Alt+7` aleg direct o formă. În funcție de formă, bara arată **Gol** și **Grosime:**, **Înălțimea treptei:** și **Direcție:** pentru scări, **Axă:**, **Laturi:** și modul cercului pentru formele rotunde, **Inele:** pentru sfere și **Precizie:** pentru icosaedru. Setează opțiunile înainte să tragi. Verifică rezultatul în mai multe vizualizări.

## Selectarea
Fă clic pe un brush pentru a-l selecta și `Ctrl`-clic pentru a-l adăuga la selecție. O tragere dintr-un spațiu gol desenează un dreptunghi de selecție. În vizualizarea 3D, `Shift`-clic selectează o singură față, iar `Ctrl+Shift`-clic adaugă alte fețe. Tragerea unui brush selectat îl mută; tragerea unuia neselectat selectează și mută doar brush-ul respectiv.

În vizualizarea 3D, `Shift`-tragerea unei fețe a unui brush selectat o extrudează. `Alt`-clic pe altă față copiază pe ea materialul și alinierea feței selectate; `Alt+Ctrl` copiază doar materialul, iar `Alt+Shift` copiază rotația.

## Unelte de modelare
- **Selectează** (`Esc`): selectează și mută.
- **Vârf** (`V`), **Muchie** (`E`), **Față** (`F`): trag colțuri, muchii sau fețe. `Shift`-tragerea din mijlocul unei muchii sau al unei fețe adaugă acolo un colț. `Delete` elimină colțurile alese.
- **Rotește** (`R`), **Scalare** (`T`), **Forfecare** (`G`): transformă selecția cu ajutorul mânerelor.
- **Tăiere** (`C`): fă clic pe două sau trei puncte pentru a defini un plan de tăiere. `Enter` taie, `Ctrl+Enter` alege partea care se păstrează, iar `Backspace` sau `Esc` elimină ultimul punct.
- **Asamblare** (`N`): plasează puncte în jurul unui volum și apasă `Enter` pentru a face din ele un brush convex. `Backspace` elimină ultimul punct.
- **Baleiere** (`Y`): vezi secțiunea Baleiere de mai jos.
- **Punct de control** (`P`): remodelează grila de control a unui patch.
- **Rotunjire** (`O`): fă clic pe o muchie pentru a o rotunji sau alege muchii ori vârfuri cu uneltele lor și apasă `Enter`. Mai multe fețe dau un rezultat mai neted și mai multă geometrie.
- **Vopsea** (`K`): vezi Materiale și UV-uri.

## Combinarea brush-urilor
- `Ctrl+J`: unește brush-urile selectate în învelitoarea lor convexă.
- `Ctrl+K`: scade brush-urile selectate din brush-urile pe care le ating.
- `Ctrl+Shift+K`: golește selecția, cu pereți groși de un pas de grilă.
- `Ctrl+L`: intersectează selecția.

## Baleiere, rotunjire, patch și teșire
**Baleiere** construiește geometrie de-a lungul unei căi pornind de la fețele selectate. Selectează fețele (în unealta Baleiere, cu `Alt`-clic), apoi setează **Parcurs**, **Rotire** (axă și unghi), **Scalare**, **Trepte** și **Cale** (aliniere Întreg sau Liber; Arc, Drept sau Curbă în S) în secțiunea Baleiere din fila Față și apasă **Baleiere**. Linia de sub controale spune câte brush-uri vor fi create.

Secțiunea **Rotunjire** rotunjește muchiile alese cu **Rază**, **Fețe** și **Colțuri** (Îmbinat în unghi sau Rotunjit); o singură față produce o teșitură.

**Patch** arată **Coloane** și **Rânduri** ale patch-ului selectat, cu butoane pentru a adăuga sau a elimina o pereche. Geometrie → Convertește fețele în patch-uri (`Ctrl+P`) creează patch-uri din fețele selectate.

**Retragere și teșire** lucrează pe fețele selectate: **Retragere** trage fața spre interior față de muchiile ei, iar **Adâncime** o împinge în afară (sau înăuntru, când este negativă). **Teșește fețele** le aplică pe amândouă; al treilea buton adaugă un panou sau taie o adâncitură, în funcție de semnul adâncimii. **Îmbinare colț** deschide un dialog care îmbină două brush-uri într-un colț îmbinat în unghi sau rotunjit, cu previzualizare.

## Text 3D
Geometrie → Inserează text 3D (și ultima intrare din lista de forme a uneltei Brush) construiește brush-uri din text. Folosește Heron Block, un font poligonal încorporat cu colțuri teșite, lățimi proporționale și diacritice cehe; literele mici folosesc formele majusculelor. Scrie **Text**, setează **Înălțime** (înălțimea majusculelor; diacriticele se ridică deasupra ei) și **Adâncime**, urmărește previzualizarea în timp real și apasă **Inserează**.

Dacă o operație nu poate produce o geometrie validă, ajustează-i dimensiunile sau anulează și simplifică forma.

## Matrice
Geometrie → Matrice copiază selecția și arată copiile în hartă cât timp schimbați valorile. Numerele includ originalul, iar o matrice face cel mult 1024 de copii.

- **Șir** mută fiecare copie față de cea dinainte cu **Pas X**, **Pas Y** și **Pas Z** și o rotește cu **Rotire per copie (grade)** în jurul axei alese.
- **Grilă** umple **Număr X**, **Număr Y** și **Număr Z** elemente, la distanțe **Pas X**, **Pas Y** și **Pas Z**.
- **Inel** întinde elementele pe **Unghi (grade)** în jurul unui centru aflat la **Rază (unități)** de selecție. Cu **Orientare spre centru** fiecare copie se rotește odată cu inelul.
- **Împrăștiere** pune copiile în locuri aleatorii în interiorul razei, de-a curmezișul axei alese. Același **Seed** dă aceeași așezare, iar **Rotire aleatorie** rotește fiecare copie.

Când selecția este un grup întreg, **Copii legate** face din fiecare copie un duplicat legat, deci o modificare la una ajunge la toate. **Aplică** păstrează copiile și le selectează împreună cu originalul; **Renunță** le elimină. Întreaga matrice se anulează într-un singur pas.

## Sculptarea terenului
Terenul este alcătuit din multe brush-uri mici, a căror suprafață pensula de sculptat o împinge în afară și o trage înăuntru. Selectează unul sau mai multe brush-uri și alege Geometrie → Împarte în coloane. Brush-urile sunt tăiate de-a lungul liniilor grilei în coloane late de un pas al grilei, puse într-un grup numit Terrain. Tăieturile urmează grila lumii, așa că brush-urile alăturate sunt tăiate în aceleași locuri și au colțuri comune. Fiecare coloană stă de-a lungul laturii subțiri a brush-ului: vertical într-o podea sau într-un tavan, orizontal într-un perete. O peșteră se face la fel: golește o cutie (Hollow selection) cu pereți groși de un pas al grilei, selectează pereții și împarte-i.

Fereastra **Sculptare**, deschisă de butonul de la capătul barei de sculptare, alege forma coloanelor și poate și împărți selecția. **Triunghi** face două coloane pe celula de grilă, iar vârful unui triunghi rămâne plan oricum l-ai sculpta. **Pătrat** face una pe celulă, deci de două ori mai puține brush-uri; vârful ei se îndoaie de-a lungul unei diagonale. **Hexagon** acoperă brush-ul cu hexagoane late de doi pași de grilă, fiecare împărțit în șase triunghiuri în jurul centrului, așa că fiecare vârf rămâne plan.

Alege unealta **Sculptare** (`J`) și trage peste suprafață în vizualizarea 3D. Inelul portocaliu arată pensula. Fără selecție, pensula schimbă fiecare brush de sub ea; cu o selecție, doar pe cele selectate. Bara din partea de sus a ferestrei alege ce face o trăsătură:

- **Ridică** și **Coboară** mută suprafața în afară sau înăuntru, cel mai mult în centrul inelului și deloc la margine. Ține `Shift` pentru a le inversa.
- **Netezește** netezește denivelările.
- **Aplatizează** aduce suprafața la nivelul unde a început trăsătura.

În aceeași fereastră, **Direcție** alege încotro:

- **Sus** mută suprafața drept în sus și în jos, într-un cerc pe sol. Aceasta e pentru teren.
- **Cameră** o mută spre vizualizare și dinspre ea, într-o sferă în jurul cursorului. Uită-te la o podea, un perete sau un tavan, iar pensula modelează ce vezi.
- **Normală** o mută în afara feței de sub cursor, acolo unde începe trăsătura.

Cele două numere sunt **Rază (unități)** și **Putere (%)**. Se mișcă doar colțurile de pe partea pe care o mișcă trăsătura; spatele rămâne pe loc, iar o coloană rămâne groasă de cel puțin o unitate. Unde brush-urile au un colț comun, îl mută împreună, așa că suprafața rămâne închisă. O trăsătură se anulează într-un pas. În vizualizările 2D unealta selectează la fel ca unealta Selectare.

Când ai terminat de sculptat, selectează terenul și alege Geometrie → **Unește coloanele** sau butonul din fereastra **Sculptare**. Coloanele vecine care împreună formează un brush convex și arată la fel devin un singur brush, așa că zonele plane rămân câteva brush-uri mari. Suprafața nu se mișcă, iar unirea se anulează într-un singur pas.

Pentru un teren ca în jocurile voxel, împarte în coloane **Pătrat** și activează **Blocuri** în fereastra **Sculptare**. O trăsătură mută atunci tot vârful fiecărei coloane cu pași întregi de grilă și lasă laturile verticale, așa că terenul urcă în trepte. Toate modurile funcționează ca înainte; atingerile mici se adună până la un pas întreg. Coloanele de alte forme rămân neschimbate.

# Materiale și UV-uri
## Încărcarea și aplicarea materialelor
Deschide secțiunea **Materiale** din fila Față. **Încarcă dosar** adaugă un dosar cu imagini, iar **Încarcă .wad** adaugă un pachet de texturi; amândouă apar sub **Colecții**, unde crucea elimină una dintre ele. Scrie în **Filtrează materialele** pentru a restrânge lista. Fă clic pe un material pentru a-l face curent, apoi **Aplică selecției** îl pune pe fiecare față selectată. **Selectează** oferă Selectează brush-urile cu acesta și Selectează fețele cu acesta. **Înlocuiește** schimbă un material cu altul în toată harta sau doar în selecție.

Unealta **Vopsea** (`K`) pune materialul curent pe fața de sub cursor: un clic vopsește o față, un dublu clic vopsește tot brush-ul, iar `Shift`-clic preia materialul feței ca material curent, ca o pipetă.

## Editorul UV
Secțiunea **UV** din fila Față arată materialul și conturul feței selectate. Controalele mouse-ului pe suprafața de lucru:
- Rotița mărește și micșorează în jurul cursorului. Trage cu butonul drept sau cu cel din mijloc pentru a deplasa vederea. Dublu clic cu butonul drept încadrează din nou fața; selectarea altei fețe o încadrează automat.
- Tragerea cu butonul stâng mută materialul, în timp ce conturul feței rămâne pe loc. Inelul rotește maparea, mânerele din colțuri o scalează, iar mânerul central stabilește originea acestor modificări.
- `Alt`-tragerea cu butonul stâng pe marginea unei dale forfecă, la fel ca `Shift`-tragerea cu butonul din mijloc.
- Fiecare tragere este un singur pas de anulare, iar `Esc` anulează o tragere în curs.

Sub suprafața de lucru sunt **Decalaj**, **Scalare**, **Unghi** și, când jocul le folosește, **Indicatori de conținut**, **Indicatori de suprafață** și **Valoare suprafață**. Butoanele sunt **Resetează**, **Resetează la lume**, **Potrivire**, **Oglindire O**, **Oglindire V** și cele două rotiri cu 90°. **Aliniere** rotește materialul de-a lungul uneia dintre muchiile feței (apasă din nou pentru muchia următoare, cu `Shift` pentru cea anterioară). Cele patru butoane de aliniere împing materialul spre o margine. **Potrivire U** și **Potrivire V** scalează astfel încât un număr întreg de copii să acopere fața; apasă din nou pentru numărul următor și ține `Ctrl` pentru a păstra subdiviziunea unui trim sheet.

## Taste UV
Cu fețe selectate, tastele săgeți mută materialul, `[` și `]` îl rotesc cu 15°, `,` și `.` îi înjumătățesc sau îi dublează scara, `Ctrl+F` și `Ctrl+Alt+F` îl oglindesc, `Shift+R` îl resetează, `Alt+Shift+R` îl resetează la axele lumii, iar `Ctrl+Shift+F` îl potrivește pe fiecare față.

**Blochează** din bara de unelte păstrează maparea când brush-urile se mută, iar **UV** (`U`) o păstrează când brush-urile se rotesc.

## Materiale unealtă
- skip: elimină fața marcată din exporturile pentru motoare.
- clip: creează coliziune fără geometrie vizibilă.
- origin: marchează pivotul unei entități brush.

Numele pot fi schimbate în cardul **Materiale unealtă** din fereastra de export. Folosește skip pentru fețele despre care știi că sunt ascunse.

## Omiterea fețelor din export
Selectează o față și activează **Omite fața din export** în secțiunea UV. O hașură portocalie o marchează în previzualizarea UV. Materialul și vopseaua ei rămân salvate, iar brush-ul rămâne închis și editabil. Mesh-urile de randare exportate omit acea față, dar coliziunea se păstrează. Dacă lași un singur quad dintr-un cub, rezultă două triunghiuri de randare. Aceasta nu face fețele cu două părți și nu unește mesh-uri.

**Omite fețele acoperite** marchează fețele acoperite complet de brush-uri statice, inclusiv fețele identice care se ating și brush-urile închise în altele. Lucrează pe brush-urile selectate sau, când nu este nimic selectat, pe toată geometria de lume vizibilă și deblocată. Entitățile brush, materialele transparente și brush-urile care au deja fețe omise sunt sărite. Fețele acoperite parțial nu sunt împărțite. O singură anulare restaurează totul. Vizualizare → Arată fețele omise ascunde sau arată astfel de fețe în vizualizările editorului; selectarea folosește în continuare întregul brush.

# Culoare și gradiente
Secțiunea **Culoare** din fila Față vopsește fețele cu culori plate sau gradiente peste materialele lor. Este utilă pentru schițarea unui nivel și pentru grafica stilizată. Fețele vopsite își păstrează culorile reale în vizualizări; selecția se vede după contur.

## Paleta și culoarea de amestec
Fă clic pe una dintre cele douăsprezece mostre ale paletei pentru a vopsi cu ea fețele selectate sau brush-urile selectate întregi. **Opacitate** stabilește, în procente, cât de opacă este următoarea vopsire. **Amestecare** are un buton de culoare care deschide selectorul de culori; trage în selector și selecția îl urmează în timp real, sau apasă **Vopsea** pentru a aplica culoarea de amestec într-un singur pas. **Înapoi la material** elimină vopseaua, astfel încât fața să-și arate din nou materialul. Linia din partea de sus a secțiunii spune ce este selectat și ce poartă.

## Selectorul de culori
Fiecare buton de culoare din editor deschide același selector. Trage în pătrat pentru a alege saturația și luminozitatea, iar în banda de lângă el pentru a alege nuanța. Dedesubt sunt valorile **R**, **G** și **B** și un glisor **Opacitate** acolo unde culoarea are unul. Câmpul hex acceptă `#RRGGBB` sau `#RRGGBBAA`; devine roșu cât timp textul nu este o culoare validă. **Copiază** pune codul hex în clipboard. Fă clic în afara selectorului pentru a-l închide.

## Gradiente
**Gradient** alege Gradient liniar sau Gradient radial, iar butonul de lângă el inversează capetele culorilor. Fiecare oprire are un buton de culoare și **Poziție (%)**, iar opririle dintre capete au un buton de eliminare. **Adaugă oprire de culoare** adaugă o oprire în cel mai mare spațiu liber; un gradient poate avea până la opt opriri. **Unghi gradient** stabilește direcția unui gradient liniar, cu butoanele **+45°** și **−45°**.

Sub previzualizare este o șină cu câte un marcaj pentru fiecare oprire. Fă clic pe șină pentru a adăuga o oprire, clic pe un marcaj pentru a-i deschide selectorul, trage un marcaj pentru a-l muta și fă clic dreapta pe un marcaj pentru a-l elimina. Cele trei mostre de sub șină sunt presetări: Gradient cald, Gradient rece și Gradient în tonuri de gri. Modificările unui gradient se aplică selecției în timp real.

Direcția unui gradient contează: orientează-l de-a lungul pantei unui acoperiș sau de-a lungul unui obiect curbat, nu peste fețe care nu au legătură între ele. După vopsire, verifică modelul din ambele părți.

## Materialul suprafeței
**Material de suprafață** stabilește **Rugozitate** (0 este lucios, 100 este mat), **Metalic** (0 pentru lemn, piatră sau plastic, 100 pentru metal) și **Emisie** (de la 0 la 8; suprafața strălucește în propria culoare, inclusiv în gradiente, dar nu luminează obiectele din apropiere). Modificările se aplică în timp real fețelor sau brush-urilor selectate. **Folosește suprafața implicită** elimină suprascrierea. Toate modificările pot fi anulate.

Efectele de suprafață se văd în vizualizarea 3D iluminată și în randările camerei. Afișarea plată și cea wireframe omit intenționat reflexele. Bloom-ul poate adăuga o strălucire în jurul suprafețelor emisive. Godot și glTF/GLB primesc valorile materialului, iar importatorul Unity transformă rugozitatea în netezime. Vezi `docs/surface-materials.md` pentru limitele fiecărei ținte.

# Asset-uri, straturi și grupuri
## Biblioteca de asset-uri
Deschide fila **Asset-uri**. Adaugă un dosar de modele `.map` cu **Adaugă dosar**; biblioteca încorporată este mereu acolo. Scrie în **Filtrează asset-urile** pentru a căuta. Fă clic pe un model pentru a-l selecta, apoi dublu clic pe el, apasă `Enter` sau fă clic pe **Inserează** pentru a-l adăuga acolo unde a fost ultima dată cursorul. Poți și trage un model într-o vizualizare. Brush-urile unui asset sunt inserate ca un singur grup, astfel încât modelul se mută ca întreg.

**Salvează selecția ca asset** scrie brush-urile selectate în bibliotecă ca fișier `.map`. Asset-urile din bibliotecă sunt brush-uri obișnuite, editabile, nu mesh-uri importate.

## Straturi
Lista **Straturi** din fila Hartă organizează zonele mari. Scrie un nume și fă clic pe **Adaugă** pentru a crea un strat sau apasă `Enter` pentru a-l redenumi pe cel curent. Fiecare strat poate fi ascuns, blocat sau exclus din exporturi. **Mută aici** mută selecția în stratul curent. Meniul contextual din vizualizări oferă și Mută în strat, Fă stratul activ, Ascunde straturile și Izolează straturile.

## Grupuri și grupuri legate
`Ctrl+G` grupează selecția, iar `Ctrl+Shift+G` o degrupează. Secțiunea **Grupuri** din fila Hartă listează grupurile și oferă **Deschide**, **Redenumește**, **Îmbină**, **Degrupează**, **Scoate** și **Extrage**. Un grup legat este un set de copii care împart modificările: `Ctrl+Shift+D` creează un duplicat legat, Geometrie → Sincronizează grupurile legate actualizează copiile, iar **Dezleagă** face o copie independentă. În fila Entitate, o proprietate poate fi păstrată ca proprie unei copii atunci când grupurile legate se sincronizează.

## O scară comună
Scara implicită de export este de 32 de unități de hartă pe metru. Marcajul încorporat pentru punctul de start al jucătorului are 64 de unități înălțime, o referință de 2 metri. Compară cu el ușile, vehiculele, mobila și plantele. Schimbă scara de export doar intenționat și păstrează coerența între asset-urile înrudite.

# Entități și gameplay
Definițiile de entități uzuale sunt încorporate și mereu disponibile: lumini, puncte de start ale jucătorului, uși, butoane, platforme, declanșatoare, obiecte statice, sunete și puncte de traseu. Încărcarea unui FGD de proiect cu **Definiții de entități** din fila Entitate înlocuiește clasele cu același nume și le păstrează pe celelalte încorporate.

## Plasarea entităților
În browserul **Entități** din fila Entitate, scrie în **Filtrează clasele** și alege o clasă. Pentru o clasă punct, cum ar fi o lumină, fă clic pe **Plasează** sau dublu clic pe clasă; `Insert` plasează una în centrul selecției. Pentru o clasă brush, cum ar fi o ușă sau un declanșator, selectează brush-urile și fă clic pe **Creează**. Clasele pot fi și trase într-o vizualizare.

## Proprietăți
Lista de proprietăți editează entitatea selectată. Entitățile din aceeași clasă sunt editate împreună. **Adaugă proprietate** adaugă o cheie pe care definiția poate să nu o cunoască; **Implicite** poate readuce proprietățile existente la valorile implicite sau poate adăuga proprietățile lipsă. Proprietățile pe care clasa le definește, dar entitatea nu le are, apar estompate; introducerea unei valori le adaugă. Folosește targetname și target pentru a lega entitățile.

## Ce poate face un export
Directivele Godot creează noduri native, corpuri de coliziune și zone de declanșare. Unity și Unreal suportă lumini uzuale native, iar Unreal creează și actori PlayerStart. Celelalte date sunt păstrate atât cât permite importatorul țintei.

O entitate ușă singură nu adaugă comportament de joc. Mișcarea, reacțiile la declanșatoare, controlerele jucătorului și scripturile proiectului trebuie lucrate în continuare în motor. Defold exportă proprietățile entităților și obiectele de coliziune; Flax plasează modele și entități generice. Verifică rezultatul în motorul tău înainte să construiești gameplay pe el.

# Animație
Heron Brush poate previzualiza mișcări simple și poate edita animații cu cadre cheie ale entităților brush, pe care apoi le exportă către mai multe motoare.

## Previzualizarea animațiilor
Activează **Previzualizează animațiile** din meniul Vizualizare, din meniul Animație, din meniul Setări grilă sau din secțiunea Vizualizare a filei Hartă. Redă rotația func_rotate și mișcarea dus-întors func_shuttle când start_on este activat, folosind proprietățile de axă, viteză, direcție, parcurs și așteptare ale entității, și redă clipurile cu cadre cheie setate pe Redă la pornire. Asset-urile ventilator și moară de vânt conțin deja părți care se rotesc.

Dezactivarea previzualizării restaurează poziția originală. Un clic într-o vizualizare oprește redarea înainte de selectare sau editare; navigarea camerei funcționează în continuare. Previzualizarea nu modifică harta, istoricul de anulare sau exporturile și nu rulează fizică sau logică de joc.

## Editorul de cadre cheie
Deschide-l cu Animație → Animație cu cadre cheie sau din meniul Setări grilă. Se andochează sub vizualizări sau în fereastra 3D cât timp aceasta este separată. Pictograma de fereastră din antetul lui îl mută într-o fereastră proprie; păstrează cadrul și istoricul de anulare. **Editează animația** comută vizualizările pe poziția animată, iar **Editează harta** revine la geometria de repaus. În modul animație, fiecare vizualizare arată jos **Editează animația** și numărul cadrului.

Selectează o entitate brush, un brush sau un grup de brush-uri pentru a-l anima. Un grup simplu de brush-uri are o singură animație și un singur pivot; grupurile care conțin mai multe entități au nevoie de animații separate.

## Clipuri
Selectorul **Animație** comută între clipurile cu nume. Scrie un nume în **Nume animație** și fă clic pe **Salvează ca** pentru a copia clipul curent sub acel nume; **Șterge** elimină un clip. **Translație** creează un clip `translate` fără rotație. **Animații uși** creează clipurile line `open` și `close` cu o rotire de 90 de grade în jurul pivotului curent; mută mai întâi pivotul în balama. **Redă la pornire** face clipul să ruleze la pornirea nivelului; presetarea pentru uși îl dezactivează.

## Transport și redare
Butoanele de transport sunt **Început**, **Cheia anterioară**, **Redare**, **Cheia următoare** și **Sfârșit**. **Cadru** arată și setează cadrul curent, iar **Durată (s)** setează lungimea clipului. Modul este **O dată**, **Buclă** sau **Ping-pong**.

## Cronologia
Cronologia are o riglă cu cursorul de redare și câte o pistă pentru **Poziție** (X, Y și Z setate ca cheie împreună) și pentru fiecare unghi de rotație, RX, RY și RZ. Fă clic sau trage pe riglă pentru a muta cursorul de redare și a previzualiza acea poziție în fiecare vizualizare. Trage cu butonul din mijloc pentru a deplasa cronologia. Trage cheile pentru a le muta în timp; `Shift` adaugă la selecție; dublu clic pe o pistă inserează o cheie; `Ctrl`+rotiță mărește sau micșorează. **Curbe** arată curbele valorilor în locul pistelor.

Butoanele de deasupra ei sunt **Adaugă cheie**, **Elimină**, **Copiază**, **Lipește** și **Duplică**. **Aliniază la cadre** ține cheile pe cadre întregi, iar **Potrivire** arată tot clipul. **Retemporizează cheile** deschide **Scară de timp**, **Decalaj (cadre)**, **Scalează de la cursorul de redare**, **Extinde durata** și **Retemporizează selecția**.

Inspectorul de chei de lângă cronologie editează cheia selectată: **Cadru**, **Valoare** (sau cele trei coordonate ale unei chei de Poziție), interpolarea **Liniar**, **Constant** sau **Bézier**, **Pantă de intrare** și **Pantă de ieșire**, precum și **Tangente netede**.

## Taste în editor
Cât timp cronologia are tastatura: `Space` redă sau pune pauză, săgețile stânga și dreapta avansează câte un cadru, `Ctrl+A` selectează toate cheile, `Ctrl+C`, `Ctrl+X` și `Ctrl+V` copiază, decupează și lipesc chei, `Ctrl+D` le duplică, `Delete` le elimină, `Ctrl+Z` anulează, iar `Esc` anulează o tragere. În afara cronologiei, `Esc` iese din modul animație. Anularea păstrează mărirea, selecția și cursorul de redare unde erau.

## Poziționarea în vizualizări
Rândul de sub cronologie conține **Straturi fantomă (onion skinning)** cu un număr și **Spațiere**, care arată pozițiile apropiate ca fantome; coordonatele **Pivot**; **Cheie automată**; și uneltele de poziționare **Translație**, **Rotește** și **Pivot**.

- **Adaugă cheie** activează Cheie automată și pune redarea pe pauză. Cu Cheie automată activată și **Translație** aleasă, trage o axă colorată a mânerului entității selectate în vizualizarea 3D pentru a seta ca cheie poziția ei la cadrul curent.
- Într-o vizualizare 2D, trage brush-ul selectat sau un membru al grupului său pentru a seta ca cheie translația de-a lungul celor două axe vizibile, cu aliniere la grilă. Pentru aceasta, Cheie automată nu trebuie să fie activată.
- Cu **Rotește**, trage brush-ul într-o vizualizare 2D în jurul marcajului portocaliu al pivotului. Sus setează cheie pentru RZ, Frontal pentru RY, iar Lateral pentru RX, aliniate la unghiurile grilei.
- Cu **Pivot**, trage centrul galben de rotație în orice vizualizare 2D. Se aliniază la grilă și își păstrează adâncimea. Cheile și geometria de repaus rămân neschimbate.

Calea cyan din 2D și 3D arată traseul pivotului. Fă clic pe un punct al căii pentru a-i selecta momentul și cheile sau trage-l pentru a schimba poziția; cu Cheie automată activată poți trage punctele căii și în vizualizarea 3D. Etichetele arată numerele cadrelor, iar segmentele punctate marchează salturile. `Esc` anulează oricare dintre aceste trageri, iar o singură anulare inversează o tragere întreagă.

Animațiile noi încep în repaus, cu chei la momentul zero. Creezi mișcare schimbând poziția la alt cadru. Pentru o cale ascuțită în formă de L, setează chei la început, în colț și la sfârșit și folosește Liniar cu Ping-pong.

## Conectarea butoanelor la animații
Selectează un func_button, trigger_multiple sau trigger_once. Fila Entitate arată **Declanșator animație** deasupra proprietăților. Alege o țintă din listă sau fă clic pe **Alege ținta** și apoi pe un obiect animat în orice vizualizare; `Esc` anulează alegerea. Țintele sau clipurile lipsă sunt raportate sub controale. Legăturile entităților arată săgeți către ținte când afișarea legăturilor este activată.

**Acțiune** alege ce face activarea: **Redă** pornește un clip cu nume sau animația selectată; **Comută** alternează `open` și `close` când acestea există, altfel repornește animația selectată; **Deschide** și **Închide** pornesc clipurile respective; **Pauză** menține poziția, **Reia** continuă, iar **Oprește** restaurează poziția originală. **Doar o dată** suprascrie valoarea implicită a clasei. **Întârziere repetare (s)** stabilește timpul minim dintre activări; nu o întârzie pe prima.

**Testează declanșatorul** rulează acțiunea în previzualizare fără să editeze harta sau să adauge pași de anulare. **Resetează previzualizarea** restaurează pozițiile originale și șterge starea de o singură dată și de reîncărcare. Aceasta nu simulează coliziunile jucătorului sau scripturile de joc.

## Animația în exporturi
Godot exportă resurse native AnimationPlayer și AnimationLibrary, inclusiv curbe și coliziune în mișcare. Un CharacterBody3D care intră în zona unui buton îi activează ținta. Modificările făcute acestor clipuri în Godot nu se întorc în hartă și sunt înlocuite la următorul export.

Exportul raylib adaugă `<map>_animation.h` cu un player în C și `<map>_preview.c` cu o previzualizare cu cameră liberă; urmează `<map>_raylib.md` pentru a-l compila. În previzualizare, `SPACE` pune pauză, `R` repornește clipurile implicite, iar `N` trece prin clipurile cu nume. Defold încorporează redarea în scriptul entității, cu mesajele documentate `heron_*_animation`. Build Level din Unity adaugă `HeronBrushAnimation` și un corp cinematic. Exportul Bevy include `heron_animation.rs` și instrucțiuni. Godot, Unity, Defold, Bevy și raylib păstrează legăturile declanșatoarelor, indicatorul de o singură dată și întârzierea de repetare. Jocul tău trebuie totuși să înregistreze playerul și să-și conecteze propriile butoane la API-ul fiecărui runtime.

## Exemplul Motion lab
Deschide `motion_lab` din Exemplele de pe ecranul de bun venit. Conține o platformă pe o cale în L, un lift, o platformă rotativă și o ușă cu balamale, cu butoane. Deschide în Godot fișierul `godot/project.godot` de lângă el și apasă `F5`. WASD și mouse-ul mișcă jucătorul, Space sare, 1–4 te duc la stații, iar R repornește stația curentă. Calcă pe un pad colorat pentru a-i activa ținta. README-ul inclus explică reexportarea.

# Exportul către un motor
Fișier → Exportă deschide fereastra de export. Motoarele sunt listate în stânga, cu siglele lor: Godot, Defold, three.js, Bevy, raylib, Unity, Unreal Engine, Flax Engine, Cocos Creator, Wavefront .obj, glTF .gltf, GLB .glb și, la final, `.map`. Pagina din dreapta spune ce va primi ținta. **Scrie** scrie exportul, **Verifică** construiește nivelul fără să-l scrie și listează problemele, iar **Închide** închide fereastra.

Exportul este separat de salvare: fișierul `.map` rămâne sursa ta editabilă. Fiecare control salvează setările motorului său imediat ce se schimbă, așa că fiecare motor își păstrează propriile răspunsuri, iar fereastra se deschide așa cum ai lăsat-o.

## Opțiuni comune
- **Scalare**: **Unități pe metru**, implicit 32.
- **Ce se scrie**: **Geometrie de coliziune**.
- **Optimizări**: **Grupează mesh-urile statice în celule de 1024 de unități**, **Elimină fețele îngropate în geometrie**, **Unește fețele care au un plan comun** și **Unește formele de coliziune**.
- **Căi proiect**: unde ajung nivelul și resursele lui în interiorul proiectului.
- **Materiale unealtă**: numele materialelor clip, skip și origin.
- **Mai mult**: comutatoare specifice motorului, precum **Denumește nodurile după**, **Reconstruiește straturile și grupurile ca noduri părinte**, **Inversează sensul triunghiurilor**, oglindirea coordonatelor de textură și un al doilea set UV pentru lumina precalculată, cu **Texel de lightmap**.
- **Ține pasul cu harta**: **Scrie din nou la fiecare salvare a hărții**. Devine disponibil după ce motorul a fost exportat o dată, pentru că acel export stabilește unde se scrie.

Celulele statice atribuie brush-urile lumii după centrele lor și le combină după material în cadrul fiecărei celule de 1024 de unități, astfel încât motoarele pot ascunde (cull) părți mai mici ale hărții. Coliziunea se păstrează, iar entitățile de gameplay rămân separate. Poate însemna mai multe apeluri de desenare decât un singur mesh al lumii, așa că opțiunea este dezactivată implicit. Exporturile OBJ, glTF și GLB de sine stătătoare nu folosesc celule.

## Note despre ținte
- Godot: căile `res://` ale texturilor și materialelor, stratul fizic al declanșatoarelor și UV2 pentru lightmap. Alege un dosar de export în interiorul proiectului Godot.
- Defold: directorul de resurse al asset-urilor, materialul modelului și samplerul de textură, care trebuie să existe în materialul ales.
- Unity: mesh-uri OBJ și un importator pentru editor; exportă în dosarul Assets al proiectului.
- Unreal Engine: un script de import generat și o cale de conținut, de obicei /Game/<level>.
- Flax Engine: o cale de conținut pentru modele, relativă la dosarul Content al proiectului.
- three.js, Bevy, raylib și Cocos Creator: scene sau cod pentru aceste runtime-uri, cu instrucțiuni lângă export.
- glTF / GLB: geometrie portabilă pentru unelte compatibile; GLB este forma binară. OBJ: geometrie și materiale pentru unelte de modelare.
- `.map`: **Scrie ca** alege Valve 220, Standard, Quake 2 sau Quake 2 (Valve). Fereastra spune ce s-ar pierde în dialectul ales.

După scriere, citește avertismentele, de exemplu despre texturi lipsă. **Ce a găsit verificarea** le listează în fereastră.

# Legături live: Godot, Unity, Defold și Flax
O legătură live trimite modificările nesalvate din Heron Brush într-un editor de motor în timp ce lucrezi. Funcționează într-un singur sens: modificările făcute în motor nu se întorc. Pentru fiecare fereastră de hartă poate rula o singură legătură live. Oprește-o înainte să schimbi motorul și repornește-o pentru a aplica opțiunile de export modificate. Controalele sunt în fereastra de export, pe pagina motorului, în cardul **Legătură live cu …**.

Indicatorul din bara de titlu arată starea legăturii; fă clic pe el pentru a opri legătura. Închiderea sau înlocuirea hărții oprește publicarea. După un import eșuat, rezolvă problema și pornește din nou legătura.

## Godot
1. Exportă o dată într-un proiect Godot.
2. Fă clic pe **Instalează addonul Godot**.
3. Activează Heron Brush Live în Project Settings → Plugins din Godot.
4. Fă clic pe **Pornește legătura live** și atașează harta dintr-un panou Godot la o scenă 3D deschisă.

Addonul înlocuiește doar ramura generată de el și păstrează poziționarea atașării și celelalte noduri. Salvează singur scena Godot și ține-ți scripturile și nodurile proprii în afara ramurii generate.

## Unity
1. Exportă o dată în dosarul Assets al proiectului Unity.
2. Fă clic pe **Instalează addonul Unity** și lasă Unity să-l compileze.
3. Deschide în Unity Window → Heron Brush → Live Link.
4. Pornește legătura și atașează harta la scena Unity activă.

Ține deschisă fereastra live din Unity. Actualizările se opresc temporar în Play Mode și Prefab Mode. Se înlocuiește doar copilul Generated al obiectului hărții. Nu edita fișierele generate.

## Defold
1. Exportă o dată în proiectul tău Defold și ține proiectul deschis în Defold.
2. Fă clic pe **Pornește legătura live**. Nu este nevoie de niciun addon.
3. Deschide heron_brush_live/preview.collection în Defold sau fă referire la ea din propria scenă.

Legătura scrie într-un dosar generat separat și lasă neatinse exportul final și game.project. **Reîncărcare la cald a jocului pornit** actualizează și un joc pornit din Defold; adăugarea de obiecte sau schimbarea coliziunii poate necesita repornirea nivelului. Conexiunile eșuate sunt reîncercate automat.

## Flax
Exportă în dosarul Content al proiectului și fă clic pe **Instalează addonul Flax**. Lasă Flax să-l compileze, pornește legătura, apoi alege Tools → Heron Brush: attach live map în scena Flax deschisă. Rădăcina generată își păstrează poziționarea, iar celelalte obiecte rămân neatinse. Salvează singur scena. Actualizările se opresc temporar în Play Mode. Tools → Heron Brush: stop live map oprește temporar recepția.

# Sesiuni live colaborative
Colaborare → Sesiune live deschide o fereastră pentru a găzdui un alt utilizator Heron Brush sau pentru a te alătura lui. Aceasta împarte editarea hărții între oameni; este separată de legătura live cu un motor.

## Găzduire și alăturare
Introdu **Numele tău**. Pentru a găzdui, alege **Găzduiește o sesiune**, fă clic pe **Pornește sesiunea** și trimite invitaților detaliile din **Copiază detaliile de conectare**. Pentru a te alătura, alege **Alătură-te unei sesiuni**, lipește detaliile și fă clic pe **Unește**. Alăturarea înlocuiește harta deschisă cu harta gazdei, așa că salvează-ți mai întâi munca.

Versiunea standard funcționează în rețeaua locală, iar **Port în rețeaua locală** setează portul. Sesiunile prin internet (**Conectare prin internet**) au nevoie de o versiune cu funcția opțională de rețea. Participanții își văd unii altora marcajele camerelor și numele în vizualizarea 3D.

Închiderea ferestrei lasă sesiunea să ruleze; indicatorul din bara de titlu îi arată starea. **Oprește sesiunea** sau **Părăsește sesiunea** o încheie. Când doi oameni editează același obiect, ordinea gazdei decide versiunea finală, iar anularea este și ea o modificare comună. Lucrați pe zone separate unde se poate și salvați puncte de control.

# Afișarea 3D și un al doilea monitor
## Moduri de afișare
Secțiunea **Vizualizare** din fila Hartă comută ce desenează vizualizările: Brush-uri, Entități punct, Modele de entități, Nume de clase ale entităților, Contururi grupuri, Patch-uri, Materiale, Arată fețele omise, Previzualizează animațiile, Wireframe, Muchii și Contururi entități brush. Wireframe desenează muchiile triunghiurilor, inclusiv diagonalele și muchiile din spatele suprafețelor, fără materiale. Este o vizualizare temporară de depanare și nu afectează exporturile sau randările.

Iluminarea, umbrele, ocluzia ambientală, reflexiile, bloom-ul și netezirea pentru vizualizarea 3D sunt în Preferințe → Motor de randare. Acestea sunt setări de previzualizare; imaginile finale folosesc setările proprii ale camerei de randare.

## O fereastră 3D separată
Vizualizare → Vizualizare 3D în fereastră separată mută vizualizarea 3D într-o fereastră proprie, pe care o poți pune pe alt monitor. Împarte camera, documentul și uneltele, iar vizualizările plane ocupă spațiul din fereastra principală. Meniul ei oferă Materiale, Vizualizare solidă și Wireframe; un submeniu Vizualizare cu Arată fețele omise, Previzualizează animațiile, Muchii, Contururi grupuri, Entități punct, Modele de entități și Contururi entități brush; Focalizează pe selecție; Frontal, Lateral și Sus, care rotesc camera în jurul selecției (sau al unui punct din fața camerei); Plasează camera din vizualizarea 3D; Ecran complet (`F11`); Randează imaginea; și Înapoi la fereastra principală. Închiderea ferestrei andochează din nou vizualizarea.

## Cerul procedural
Cerul procedural este activat implicit în vizualizarea 3D și poate fi dezactivat în Preferințe → Motor de randare → Lumină. Camerele de randare au propriul comutator **Cer procedural**, salvat împreună cu harta. Culorile cerului, orizontului și solului vin din setările de mediu, iar soarele urmează iluminarea scenei. Cerul nu are geometrie și nu este exportat.

# Camere de randare și imagini PNG
Fișier → Randează imaginea deschide fereastra de randare: camera și setările ei în stânga și o previzualizare în timp real în dreapta, care se redesenează pe măsură ce le schimbi. Camerele sunt salvate în fișierul `.map`, într-un strat al camerelor de randare folosit doar de editor, care este exclus din exporturile pentru motoare.

## Plasarea camerei
Încadrează cadrul în vizualizarea 3D și apasă `Ctrl+Alt+0` (sau `Ctrl+Alt+Num0`) ori fă clic pe **Plasează camera din vizualizarea 3D**. Dacă nicio cameră nu este selectată, se actualizează prima cameră salvată; dacă nu există niciuna, se creează una. Operația poate fi anulată și păstrează setările de ieșire și de iluminare. **Selectează camera din hartă**, **Încarcă camera din hartă** și **Mută vizualizarea 3D la cameră** lucrează în sens invers. **Cadrul camerei în vizualizarea 3D** arată limitele imaginii de ieșire, iar **Controlează camera din vizualizarea 3D** permite navigării obișnuite să miște camera salvată.

## Setări
- **Poziția camerei**: X, Y, Z, girație și înclinare.
- **Ieșire și obiectiv**: **Presetări** (Full HD, Pătrat, Portret), lățime, înălțime, eșantioane, **Câmp vizual vertical** și **Plan de tăiere îndepărtat**.
- **Iluminare**: Lumină de zi, Apus sau Înnorat, ori setează expunerea, soarele, culorile cerului, orizontului și solului, **Cer procedural**, **Umbre**, **Ocluzie ambientală** și **Strălucire (bloom)**.
- **Profunzime de câmp**: **Activează profunzimea de câmp**, distanța de focalizare, intervalul clar și raza de estompare. **Focalizează pe selecție** focalizează pe centrul unui obiect selectat aflat în fața camerei.

**Salvează camera și setările** stochează formularul în hartă; salvează harta pentru a-l păstra pe disc.

## Randarea
**Randează PNG** întreabă unde să salveze și pornește o randare progresivă, care se completează eșantion cu eșantion în previzualizare. Mai multe eșantioane arată mai bine, dar durează mai mult. **Anulează randarea** se oprește fără să scrie un fișier. Motorul de randare folosește pipeline-ul raster al editorului, nu path tracing. PNG-ul are fundal opac, iar animațiile nu sunt randate. Imaginile foarte mari pot bloca scurt editorul.

# Preferințe
Fișier → Preferințe (`Ctrl+,`) deschide setările. Modificările se aplică imediat și sunt salvate. Bara laterală listează Prezentare generală, Jocuri, Vizualizare, Culori, Mouse, Tastatură și Motor de randare. Câmpul de căutare din partea de sus găsește o setare după nume sau descriere pe orice pagină. Fiecare setare modificată are propriul buton pentru restaurarea valorii implicite, iar **Restaurează valorile implicite** resetează pagina curentă.

## Paginile
- **Prezentare generală**: setările uzuale la un loc: Aspect (Opacitate panou (%), Schemă de culori, Aranjament, Limbă), Editare, Vizualizare și Manual de utilizare.
- **Jocuri**: jocul, unde se află fișierele lui și numele tău pentru colaborare. Un joc spune ce format de hartă se scrie, ce definiții de entități se încarcă și unde îi sunt materialele. Configurațiile jocurilor sunt citite din dosarul `games` din dosarul de configurare (`~/.config/heron-brush/games` pe Linux și macOS, `%LOCALAPPDATA%\heron-brush\games` pe Windows) și din dosarul `games` de lângă aplicație, unde pachetele pun configurația inclusă.
- **Vizualizare**: Aranjament, Schemă de culori, Limbă, axele lumii, opacitatea grilei, dimensiunea celulei pentru verificarea scurgerilor și blocările de mapare.
- **Culori**: culorile vizualizărilor, geometriei și mânerelor.
- **Mouse**: vitezele și inversările pentru privire, deplasare și mișcare, viteza de zbor, câmpul vizual, Leagă vizualizările 2D și dimensiunile mânerelor.
- **Tastatură**: asocierile de taste, Heron Brush sau Blender.
- **Motor de randare**: umbrire realistă, conturul selecției, lumină, umbre, ocluzie ambientală, reflexii, bloom și suprafețe.

## Limba
**Limbă** oferă Ca în sistem și fiecare limbă tradusă. Interfața se schimbă imediat. Manualul se deschide în limba interfeței când există o traducere a lui și în engleză în caz contrar.

# Pluginuri și MCP
## Pluginuri
Pluginurile extind editorul cu scripturi Rhai. Fișier → Pluginuri deschide fereastra Pluginuri. **Instalează un plugin** instalează un fișier `.rhai`, iar **Instalează un dosar de plugin** instalează un dosar care conține `main.rhai`; metadatele opționale din `plugin.toml` descriu pluginul și setările lui. Pluginurile sunt grupate sub **Pluginuri active** și **Pluginuri inactive**, iar **Activează pluginul** pornește sau oprește un plugin; alegerea rămâne valabilă și după repornire. **Reîncarcă pluginurile** citește din nou scripturile de pe disc și le resetează valorile din panou.

Pluginurile active apar și în secțiunea **Pluginuri** din fila Hartă, cu setările lor, butoanele de acțiune (**Rulează**) și ieșirea. O acțiune lucrează pe selecție sau, când nu este nimic selectat, pe tot și reprezintă un singur pas de anulare. Inspectorul de selecție inclus arată numărători și acțiuni de mutare în sus și în jos.

## MCP
Colaborare → Servește pentru un agent (MCP) permite unui agent AI sau altei unelte să controleze editorul deschis prin Model Context Protocol. Fereastra arată adresa locală cu **Copiază**, iar **Oprește serverul** îl închide; cât timp rulează, bara de titlu arată **MCP activ**. Opțiunea de linie de comandă `--mcp` rulează aceeași interfață fără fereastră. Clienții pot descoperi unelte pentru geometrie, asset-uri, proprietăți, animație, export și sesiuni. MCP și pluginurile editorului sunt interfețe diferite.

# Linia de comandă
Programul este `heron-brush`. Fără opțiuni, deschide ecranul de bun venit.

## Deschiderea fișierelor
`heron-brush map.map [entities.fgd] [materials/]` deschide o hartă, opțional cu definiții de entități și un dosar de materiale. Un argument gol sare peste unul dintre ele, de exemplu `heron-brush map.map "" materials/`. Dacă un editor rulează deja, harta se deschide în el; `--new-instance` deschide în schimb o fereastră separată.

## Comenzi
- `--help` sau `-h`: afișează modul de utilizare. `--version` sau `-V`: afișează versiunea.
- `--mcp`: servește MCP prin intrarea și ieșirea standard, fără fereastră.
- `--merge-map <base> <ours> <theirs> [<output>]`: o îmbinare în trei căi a fișierelor `.map` pentru git. Ieșirea este implicit ours. Conflictele sunt afișate și dau un cod de ieșire diferit de zero.
- `--obj-to-map <in.obj> <out.map> [size] [thickness] [snap] [material]`: transformă un mesh în brush-uri. Size este lungimea celei mai lungi laturi în unități de hartă (implicit 256), snap este o grilă pentru colțuri (0 pentru niciuna), iar cuvintele `solid` sau `layered` construiesc cutii pe grilă sau plăci convexe în loc de pene subțiri.
- `--render-preview <in.map> <out.png> [size]`: scrie o miniatură PNG a unei hărți, implicit de 512 pixeli.
- `--run-script <script.rhai> <in.map> [out.map] [key=value ...]`: rulează un script de plugin pe o hartă cu totul selectat și scrie rezultatul (implicit înapoi în fișierul de intrare). `key=value` răspunde la setările pluginului.

# Depanare
## Când ceva arată greșit
- Material lipsă: verifică dosarul sau pachetul de materiale și căile proiectului din export.
- Dimensiune greșită a modelului: compară cu marcajul jucătorului de 64 de unități și cu scara de export.
- Direcție greșită a gradientului: verifică unghiul gradientului și orientarea feței.
- Scenă goală în motor: citește avertismentele exportului și urmează instrucțiunile de import ale țintei.
- Legătura live așteaptă: verifică dacă addonul este instalat, scena este atașată și actualizările nu sunt oprite temporar.
- Doar linii, fără fețe: dezactivează Wireframe în secțiunea Vizualizare din fila Hartă sau alege Materiale ori Vizualizare solidă în meniul ferestrei 3D.
- Obiecte care lipsesc din vizualizări: `Ctrl+Shift+I` arată obiectele ascunse; verifică straturile și filtrele de vizualizare.
- O tastă nu face nimic: fă mai întâi clic în vizualizare ca să primească tastatura și verifică asocierile de taste în Preferințe → Tastatură.
- O editare a eșuat: anulează, simplifică geometria și folosește **Verifică harta** și **Verifică scurgerile** în secțiunea **Probleme** din fila Hartă.
- O hartă nu se deschide din managerul de fișiere: poate a fost predată unui editor care rulează deja; caută acea fereastră.

## Setări și copii de rezervă
Preferințele, hărțile recente, stelele, setările de export și pluginurile se află în dosarul de configurare: `~/.config/heron-brush` pe Linux și macOS și `%LOCALAPPDATA%\heron-brush` pe Windows. Copiile de rezervă automate sunt în dosarul `autosave` de lângă fiecare hartă.

## Raportarea unei erori
Include versiunea editorului (Ajutor → Despre), sistemul de operare, o hartă mică ce arată problema și pașii exacți. Panoul de ieșire (`Ctrl+4`) spune adesea ce a mers prost. Salvează o copie înainte de a experimenta.

## Folosirea acestui manual
Alege un capitol din cuprinsul din stânga. `Ctrl+F` arată cuprinsul și mută cursorul în câmpul de căutare; `Esc` golește căutarea sau închide manualul când căutarea este goală. **Conținut** ascunde sau arată lista capitolelor, iar **Dimensiune text** schimbă dimensiunea fontului; ambele sunt reținute. Lista de limbi din partea de sus comută între traducerile disponibile.
