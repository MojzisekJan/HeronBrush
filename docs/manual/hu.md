# Első lépések
A Heron Brush brush-alapú pályaszerkesztő. A pályákat konvex brush-okból építi fel, a lapjaikat kifesti és anyaggal látja el, entitásokat helyez el, ajtókat és platformokat animál, az eredményt pedig játékmotorba exportálja. Beépített eszközöket, paraméteres alakzatokat, motorszerkesztőkkel való élő kapcsolatot, közös szerkesztési munkameneteket és állóképek renderelését is kínálja. Ez a kézikönyv az alkalmazás része, és internetkapcsolat nélkül is működik.

macOS-en a `Cmd` mindenhol működik, ahol ez a kézikönyv `Ctrl` billentyűt említ, az Option pedig az Alt.

## A kezdőképernyő
Ha a Heron Brush térkép nélkül indul, a kezdőképernyő nyílik meg. Bal oldalon található a **… folytatása** (az utoljára megnyitott térkép), az **Új térkép** és a **Tallózás** egy meglévő `.map` megnyitásához, valamint a **Játék**, a **Kézikönyv** és a **Beállítások**. Jobb oldalon a legutóbbi térképek listája látható bélyegképekkel, alatta a **Példák**. A csillag (**Mindig felül**) a lista tetejére tűz egy térképet; a kuka (**Áthelyezés a Lomtárba**) a rendszer lomtárába helyezi a fájlt, így az visszaállítható. A nyílbillentyűkkel lehet mozogni a képernyőn, az Enter pedig megnyitja a kijelölt elemet. A Fájl → Üdvözöljük menüponttal bármikor visszatérhet a kezdőképernyőre.

## Az első térkép
1. Válassza az **Új térkép** lehetőséget a kezdőképernyőn, vagy a Fájl → Új térkép menüpontot (`Ctrl+N`).
2. Válassza ki az eszközsávon a Brush eszközt (`B`), és húzással rajzolja meg a kiválasztott alakzatot egy 2D nézetben. A mélységét egy másik 2D nézetben ellenőrizze.
3. Jelölje ki a brush-t, és húzással mozgassa. A **Lap** lapon anyagot vagy színt adhat neki.
4. Mentse a forrástérképet a `Ctrl+S` billentyűkkel. Ha készen áll arra, hogy játékban használja, nyissa meg a Fájl → Exportálás menüpontot, és írjon ki egy jelenetet a motorja számára.

## Fájlok és mentés
A `.map` fájl a szerkeszthető forrás. Mentsen rendszeresen: az exportálás, az élő kapcsolat és a renderelés soha nem menti Ön helyett a térképet. A Fájl → Mentés másként (`Ctrl+Shift+S`) hozzáadja a `.map` kiterjesztést, ha elhagyja. A Fájl → Visszatérés a mentett térképhez elveti a nem mentett módosításokat, és újra betölti a fájlt. Ha nem mentett módosításokkal zár be egy ablakot vagy nyit meg egy másik térképet, a Heron Brush megkérdezi, hogy **Mentés**, **Elvetés** vagy **Mégse**.

Amíg egy mentett térképen nem mentett módosítások vannak, tízpercenként számozott biztonsági mentések készülnek a térkép melletti `autosave` mappába. A program az 50 legújabb mentést őrzi meg.

Ha egy térképet akkor nyit meg, amikor a Heron Brush már fut, a futó ablak kapja meg, és az előtérbe kerül. Ha külön ablakot szeretne, használja a parancssorban a `--new-instance` kapcsolót (lásd: Parancssor).

# A felület
A főablakban felül a menüket tartalmazó címsor, alatta az eszközsáv, majd a térképnézetek, jobb oldalon a vizsgáló, alul pedig az állapotsor található.

## Címsor és menük
A címsor a logót, a menüket, középen a térkép nevét és az ablakgombokat mutatja. macOS-en a megszokott piros, sárga és zöld gombok bal oldalon vannak. A címsor húzásával mozgathatja az ablakot, dupla kattintással pedig teljes méretűre nagyíthatja. Amíg valami a háttérben fut, a sáv vége közelében egy jelző jelenik meg: egy motor élő kapcsolata, egy élő munkamenet vagy az **MCP aktív**. A jelzőre kattintva leállíthatja az élő kapcsolatot, vagy megnyithatja az ablakát.

- **Fájl**: Üdvözöljük, Új térkép (`Ctrl+N`), Térkép megnyitása (`Ctrl+O`), Mentés (`Ctrl+S`), Mentés másként (`Ctrl+Shift+S`), Visszatérés a mentett térképhez, Exportálás, Kép renderelése, Bővítmények és Beállítások (`Ctrl+,`).
- **Szerkesztés**: Visszavonás (`Ctrl+Z`), Újra (`Ctrl+Shift+Z`), Kivágás, Másolás, Beillesztés, Kettőzés (`Ctrl+D`), Törlés, Összes kijelölése (`Ctrl+A`), Utolsó parancsok ismétlése (`Ctrl+R`) és Ismételhető parancsok elfelejtése.
- **Nézet**: 3D nézet külön ablakban, Ugrás, Illesztés a rácshoz (`Alt+0`), Rács megjelenítése (`0`), Animációk előnézete és Kihagyott lapok megjelenítése.
- **Geometria**: 3D szöveg beszúrása, Sarok összekötése, Lapok átalakítása patch-ekké, A kijelölés kivétele az entitásából és Csatolt csoportok szinkronizálása.
- **Animáció**: Kulcskockás animáció, Animációk előnézete és Kamera elhelyezése a 3D nézetből (`Ctrl+Alt+0`).
- **Térkép**: Pontfájl betöltése, újratöltése és eltávolítása; Portálfájl betöltése, újratöltése és eltávolítása. Ezek a térképfordítók által kiírt szivárgási nyomvonalakat és portálokat jelenítik meg.
- **Együttműködés**: Élő munkamenet és Kiszolgálás ügynöknek (MCP).
- **Súgó**: Kézikönyv és Névjegy.

## Az eszközsáv
Az eszközsáv minden eszközt tartalmaz. Az első csoportot használja a leggyakrabban: Kijelölés, Brush, Vágás, Csúcs, Él, Lap, Forgatás és Méretezés. A második csoportban a Nyírás, az Összeállítás, a Söprés, a Vezérlőpont, a Lekerekítés és a Festés található. Ha a mutatót egy gomb fölé viszi, megjelenik a neve és a billentyűparancsa. Az aktív eszközre ismét kattintva visszatér a Kijelöléshez.

Az eszközök után a **Vízszintes tükrözés** és a **Függőleges tükrözés** következik, majd két kapcsoló: a **Zárolás** megtartja az anyag leképezését, amikor egy brush elmozdul, az **UV** pedig akkor, amikor egy brush elfordul (az `U` billentyű kapcsolja). Ezután az aktív eszköz saját beállításai következnek: a Brush eszköz az alakzatot és annak beállításait mutatja, a Lekerekítés eszköz a sugarat, a lapokat és a sarkokat az **Alkalmaz** gombbal. A jobb szélen az eszköz neve, a rácsméret (**Rács 16** és így tovább; kattintással másikat választhat) és a **Rácsbeállítások** menü található, benne az Illesztés a rácshoz, a Rács megjelenítése, az Animációk előnézete, a Kihagyott lapok megjelenítése és a Kulcskockás animáció.

## Térképnézetek
Négy nézet van: a 3D nézet és három sík nézet, Felül (XY), Elöl (XZ) és Oldalt (YZ). A Beállítások → Nézet → Elrendezés lehetőségei: Egy panel, Két panel, Három panel vagy Négy panel. A panelek átméretezéséhez húzza az elválasztókat. Egy nézetbe kattintva az kapja meg a billentyűzetet. A `Space` az egyszerre egy nézetet mutató panelt a következő nézetére váltja, a `Ctrl+Space` pedig teljes méretűre nagyítja a billentyűzetet birtokló nézetet, majd visszaállítja az elrendezést.

Bármelyik nézetben húzás nélkül jobb gombbal kattintva helyi menü nyílik a kijelöléshez és a mutató alatti elemhez. Ebben csoportosítás, rétegek, csatolt csoportok, entitások létrehozása, brush-ok entitásba helyezése vagy onnan kivétele, egy anyag nevének másolása és az anyag megmutatása az anyagböngészőben érhető el.

## A vizsgáló
A jobb oldali vizsgálónak négy lapja van. A szélességét a bal szélének húzásával módosíthatja.

- **Térkép** (`Ctrl+1`): Rétegek, Csoportok, Bővítmények, Problémák, View filters és Címkék.
- **Entitás** (`Ctrl+2`): az entitásböngésző, az entitásdefiníciók, a kijelölt entitás tulajdonságai, valamint gombok és eseményindítók esetén az Animációs eseményindító.
- **Lap** (`Ctrl+3`): UV, Anyagok, Szín, Söprés, Lekerekítés, Patch, valamint Behúzás és letörés. Minden szakasz összecsukható.
- **Eszközök**: az újrafelhasználható brush-modellek könyvtára.

A `Ctrl+5` elrejti vagy megjeleníti a vizsgálót, a `Ctrl+Alt+T` pedig az eszközsávot, így a nézetek több helyet kapnak.

## Az állapotsor és a kimeneti panel
Az alsó állapotsor a szerkesztő legutóbbi üzenetét mutatja. A hibák pirossal jelennek meg. A sávra kattintva vagy a `Ctrl+4` billentyűkkel megnyílik a kimeneti panel az üzenetek teljes előzményével. A legtöbb parancs itt jelzi, mit tett, például hány brush-t vágott el.

## Párbeszédablakok és megjelenés
A párbeszédablakok, például a Beállítások, az Exportálás és a Kép renderelése, saját ablakban nyílnak meg. A Beállítások → Nézet → Színséma lehetőségei: Asztali beállítás követése, Világos vagy Sötét. Az Áttekintés oldalon található **Panel átlátszatlansága (%)** áttetszővé teszi a paneleket a nézetek fölött. Az ablak a saját gombjait a rendszer stílusában rajzolja meg: macOS-en natívan, Windowson Windows 11 stílusban, Linuxon GNOME stílusban.

# Navigáció és billentyűzet
## Mozgás a 3D nézetben
Tartsa lenyomva a jobb egérgombot, és húzással nézzen körül. A középső gombbal húzva pásztázhat. A görgő előre és hátra mozgat. A `W`, `A`, `S` és `D` előre, balra, hátra és jobbra repít, a `Q` emelkedik, az `X` süllyed; nagyobb lépésekhez tartsa lenyomva a `Shift` billentyűt. Ha a Beállítások → Egér oldalon be van kapcsolva az **Alt + középső húzás mozgatja a kamerát**, az Alt a középső gombbal a nézés irányában mozgat. A sebességek, az invertálások és a látószög a Beállítások → Egér oldalon találhatók.

## Mozgás a 2D nézetekben
A középső gombbal húzva pásztázhat, a görgővel pedig a mutató körül nagyíthat. Ha egy sík nézetet pásztáz vagy nagyít, a többi igazodik hozzá; ezt a Beállítások → Egér oldalon a **2D nézetek összekapcsolása** szabályozza. A `Ctrl+Alt+Z` visszaállítja a nagyítást, a `Ctrl+Shift+U` pedig a sík nézeteket az origóra állítja.

A `Ctrl+U` minden nézetben bekeretezi a kijelölést. A Nézet → Ugrás (`Ctrl+Alt+M`) a kamerát a beírt koordinátákra viszi, kijelöli a térképfájl adott sorában lévő elemet, vagy a kijelölést a beírt értékkel mozgatja.

## Rács és illesztés
A `+` és a `-` módosítja a rácsméretet, az `1`–`9` számbillentyűk pedig közvetlenül választanak méretet 1 és 256 egység között. A `0` vagy a `#` megjeleníti vagy elrejti a rácsot, az `Alt+0` pedig be- vagy kikapcsolja az illesztést. Az `S` a kijelölt brush-ok csúcsait az aktuális rácshoz illeszti, a `Ctrl+Shift+V` egész egységekhez, a `Ctrl+Alt+Shift+V` pedig a rácshoz.

## Gyakori billentyűparancsok
- `F3` vagy `Ctrl+Shift+P`: bármely menüparancs keresése név szerint, futtatás `Enter`-rel. Az `F10` megnyitja a menüket, a nyilakkal lehet bennük mozogni.
- `Ctrl+S`: a térkép mentése. `Ctrl+Shift+S`: mentés másként.
- `Ctrl+Z`: visszavonás. `Ctrl+Shift+Z` vagy `Ctrl+Y`: újra.
- `Ctrl+C`, `Ctrl+X`, `Ctrl+V`: másolás, kivágás és beillesztés. A beillesztés a mutatóhoz teszi az objektumokat; a `Ctrl+Alt+V` megtartja az eredeti koordinátáikat. Kijelölt lapok esetén a másolás az anyagukat és az UV-beállításaikat veszi át.
- `Ctrl+D`: kettőzés. `Ctrl+Shift+D`: egy csoport csatolt másolata.
- `Delete` vagy `Backspace`: a kijelölés törlése.
- `Esc`: megszakítja az aktuális húzást, egy lépést visszalép az aktív eszközben, visszatér a Kijelöléshez, végül megszünteti a kijelölést. A `Shift+Esc` közvetlenül a Kijelöléshez tér vissza.
- `Ctrl+R`: az utolsó ismételhető parancsok ismétlése. `Ctrl+Shift+R`: elfelejtésük.
- `Ctrl+Alt+0` vagy `Ctrl+Alt+Num0`: a renderkamera elhelyezése a 3D nézet helyére.
- `F5`: az anyagok újratöltése. `F6`: az entitásdefiníciók újratöltése.

## Kijelölési billentyűparancsok
- `Ctrl+A`: mindent kijelöl, vagy csúcs-, él- vagy lapeszköz használatakor az összes fogantyút. `Ctrl+Shift+A`: kijelölés megszüntetése. `Ctrl+Alt+A`: a kijelölés megfordítása.
- `Ctrl+B`: kijelöli az ugyanabban az entitásban vagy csoportban lévő többi objektumot.
- `Ctrl+T`: kijelöl mindent, amihez a kijelölt brush-ok hozzáérnek. `Ctrl+E`: kijelöl mindent, ami bennük van. `Ctrl+Shift+E`: kijelöl mindent, ami a nézet tengelye mentén az oszlopukban van. Ezek a parancsok felhasználják a kijelölő brush-okat.
- `Ctrl+I`: a kijelölés elkülönítése. `Ctrl+Alt+I`: elrejtése. `Ctrl+Shift+I`: minden ismételt megjelenítése.
- `Ctrl+G`: csoportosítás. `Ctrl+Shift+G`: csoport bontása.

## Átalakítási billentyűparancsok
- A nyílbillentyűk és a `Page Up` / `Page Down` egy rácslépéssel mozgatják a kijelölést abba az irányba, amerre az aktív nézetben mutatnak. A `Ctrl` billentyűvel mozgatás közben kettőznek, az `Alt` billentyűvel forgatnak.
- `Alt+X`, `Alt+Y`, `Alt+Z`: negyed fordulat az adott tengely körül; a `Shift` hozzáadásával az ellenkező irányba.
- `Ctrl+F`: tükrözés a nézeten keresztben; `Ctrl+Alt+F`: tükrözés fel-le. A `Ctrl+Shift+X` és a `Ctrl+Shift+Y` az X vagy az Y mentén tükröz.
- `Insert`: pontentitás elhelyezése a kijelölés középpontjában.
- `Tab`: kijelölt brush esetén körbevált: Kijelölés → Csúcs → Él → Lap → Kijelölés.

## A Blender billentyűkiosztás
A Beállítások → Billentyűzet → Billentyűkiosztás lehetőségei a Heron Brush (az alapértelmezett, ezt használja a kézikönyv) és a Blender. Blender kiosztással: a `G` kijelöl és mozgat, az `R` forgat, az `S` méretez, az `E` söpör, az `1`, `2` és `3` a csúcs-, él- és lapeszközt választja, a `K` vág, a `B` kijelöl, az `A` mindent kijelöl, az `Alt+A` megszünteti a kijelölést, az `X` töröl, a `Shift+D` kettőz, a `H` elrejt, az `Alt+H` mindent megjelenít, a `Ctrl+Shift+Z` pedig újra végrehajt. Minden más mindkét kiosztásban ugyanaz.

# Brush-ok és modellező eszközök
A brush-ok síkokkal határolt konvex testek. Az összetett objektumokat több egyszerű brush-ból építse fel, ne próbáljon egyetlen brush-t konkávvá tenni. Ahol egy egyszerű alakzat is elég, tartsa alacsonyan a brush-ok számát.

## Brush-ok rajzolása
Válassza a Brush eszközt (`B`), és húzzon egy nézetben. Az eszközsáv a **Rajzolandó alakzat** listát mutatja: Téglatest, Lépcső, Boltív, Henger, Kúp, Gömbtest (UV), Gömbtest (ikozaéder) és 3D szöveg beszúrása. Az `Alt+1`–`Alt+7` közvetlenül választ alakzatot. Az alakzattól függően a sáv az **Üreges** és **Vastagság:** beállítást, lépcsőnél a **Lépcsőfok magassága:** és **Irány:**, kerek alakzatoknál a **Tengely:**, **Oldalak:** beállítást és a körmódot, gömböknél a **Gyűrűk:**, ikozaédernél a **Pontosság:** beállítást mutatja. A beállításokat a húzás előtt adja meg. Az eredményt több nézetben is ellenőrizze.

## Kijelölés
Egy brush-ra kattintva kijelöli, `Ctrl`-kattintással pedig hozzáadja a kijelöléshez. Üres helyről húzva kijelölő téglalapot rajzol. A 3D nézetben a `Shift`-kattintás egyetlen lapot jelöl ki, a `Ctrl+Shift`-kattintás további lapokat ad hozzá. Egy kijelölt brush húzása mozgatja azt; egy nem kijelölt brush húzása csak azt az egyet jelöli ki és mozgatja.

A 3D nézetben egy kijelölt brush lapját `Shift`-húzással kihúzhatja. Egy másik lapra `Alt`-kattintva rámásolja a kijelölt lap anyagát és igazítását; az `Alt+Ctrl` csak az anyagot, az `Alt+Shift` a forgatást másolja.

## Alakító eszközök
- **Kijelölés** (`Esc`): kijelölés és mozgatás.
- **Csúcs** (`V`), **Él** (`E`), **Lap** (`F`): sarkok, élek vagy lapok húzása. Egy él vagy lap közepéről `Shift`-húzással ott új sarkot ad hozzá. A `Delete` eltávolítja a kiválasztott sarkokat.
- **Forgatás** (`R`), **Méretezés** (`T`), **Nyírás** (`G`): a kijelölés átalakítása fogantyúkkal.
- **Vágás** (`C`): két vagy három pontra kattintva vágósíkot határoz meg. Az `Enter` vág, a `Ctrl+Enter` kiválasztja, melyik oldal maradjon meg, a `Backspace` vagy az `Esc` pedig eltávolítja az utolsó pontot.
- **Összeállítás** (`N`): helyezzen el pontokat egy térfogat körül, és az `Enter` megnyomásával konvex brush-t készít belőlük. A `Backspace` eltávolítja az utolsó pontot.
- **Söprés** (`Y`): lásd lent a Söprés szakaszt.
- **Vezérlőpont** (`P`): egy patch vezérlőrácsának átformálása.
- **Lekerekítés** (`O`): egy élre kattintva lekerekíti, vagy válasszon ki éleket vagy csúcsokat a saját eszközeikkel, és nyomja meg az `Enter` billentyűt. Több lap simább eredményt és több geometriát ad.
- **Festés** (`K`): lásd az Anyagok és UV-k fejezetet.

## Brush-ok kombinálása
- `Ctrl+J`: a kijelölt brush-ok egyesítése a konvex burkukba.
- `Ctrl+K`: a kijelölt brush-ok kivonása azokból a brush-okból, amelyekhez hozzáérnek.
- `Ctrl+Shift+K`: a kijelölés kiüregesítése egy rácslépés vastag falakkal.
- `Ctrl+L`: a kijelölés metszete.

## Söprés, lekerekítés, patch és letörés
A **Söprés** a kijelölt lapokból egy útvonal mentén épít geometriát. Jelölje ki a lapokat (a Söprés eszközben `Alt`-kattintással), majd a Lap lap Söprés szakaszában állítsa be az **Út**, a **Forgatás** (tengely és szög), a **Méretezés**, a **Lépcsőfokok** és az **Útvonal** értékét (Egész szám vagy Szabad igazítás; Ív, Egyenes vagy S-ív), és nyomja meg a **Söprés** gombot. A vezérlők alatti sor megmondja, hány brush készül.

A **Lekerekítés** szakasz a kiválasztott éleket a **Sugár**, a **Lapok** és a **Sarkok** (Gérvágott vagy Lekerekített) beállításával kerekíti le; egy lap élletörést ad.

A **Patch** a kijelölt patch **Oszlopok** és **Sorok** értékét mutatja, egy-egy pár hozzáadására vagy elvételére szolgáló gombokkal. A Geometria → Lapok átalakítása patch-ekké (`Ctrl+P`) a kijelölt lapokból patch-eket készít.

A **Behúzás és letörés** a kijelölt lapokon működik: a **Behúzás** a lapot az éleitől befelé húzza, a **Mélység** kifelé tolja (negatív értéknél befelé). A **Lapok letörése** mindkettőt alkalmazza; a harmadik gomb a Mélység előjelétől függően panelt ad hozzá vagy mélyedést vág ki. A **Sarok összekötése** egy párbeszédablakot nyit, amely két brush-t gérvágott vagy lekerekített sarokkal köt össze, előnézettel.

## 3D szöveg
A Geometria → 3D szöveg beszúrása (a Brush eszköz alakzatlistájának utolsó eleme is) szövegből épít brush-okat. A Heron Block nevű beépített sokszögbetűtípust használja, amely levágott sarkokkal, arányos szélességekkel és cseh ékezetekkel rendelkezik; a kisbetűk nagybetűs formát kapnak. Írja be a **Szöveg** mezőbe a szöveget, állítsa be a **Magasság** (a nagybetűk magassága; az ékezetek fölé nyúlnak) és a **Mélység** értékét, figyelje az élő előnézetet, és nyomja meg a **Beszúrás** gombot.

Ha egy művelet nem tud érvényes geometriát létrehozni, módosítsa a méreteit, vagy vonja vissza, és egyszerűsítse az alakzatot.

## Tömbök
A Geometria → Tömb lemásolja a kijelölést, és a másolatokat a térképen mutatja, amíg az értékeket módosítod. A darabszámok az eredetit is tartalmazzák, és egy tömb legfeljebb 1024 másolatot készít.

- A **Sor** minden másolatot az előzőhöz képest **X lépés**, **Y lépés** és **Z lépés** értékkel tol el, és **Forgatás másolatonként (fok)** értékkel forgat a választott tengely körül.
- A **Rács** **Darabszám X**, **Darabszám Y** és **Darabszám Z** elemet tölt ki **X lépés**, **Y lépés** és **Z lépés** távolságra.
- A **Gyűrű** az elemeket **Szög (fok)** szögben osztja el egy középpont körül, amely **Sugár (egység)** távolságra van a kijelöléstől. A **Fordítás a középpont felé** bekapcsolásával minden másolat a gyűrűvel együtt fordul.
- A **Szórás** a másolatokat véletlen helyekre teszi a sugáron belül, a választott tengelyre merőlegesen. Ugyanaz a **Mag** ugyanazt az elrendezést adja, a **Véletlen forgatás** pedig minden másolatot elforgat.

Ha egy teljes csoport van kijelölve, az **Összekapcsolt másolatok** minden másolatot összekapcsolt duplikátummá tesz, így az egyiken végzett módosítás mindegyikre kihat. Az **Alkalmaz** megtartja a másolatokat, és az eredetivel együtt kijelöli őket; a **Mégse** eltávolítja őket. Az egész tömb egy lépésben visszavonható.

## Terep formázása
A terep sok kis brush-ból áll, amelyek felületét a formázó ecset kifelé tolja és befelé húzza. Jelölj ki egy vagy több brush-t, és válaszd a Geometria → Oszlopokra bontás parancsot. Ez a rácsvonalak mentén egy rácslépésnyi széles oszlopokra vágja őket, az oszlopokat pedig egy Terrain nevű csoportba teszi. A vágások a világ rácsát követik, így az egymás melletti brush-ok ugyanott vágódnak el, és közös sarkaik lesznek. Minden oszlop a brush legvékonyabb oldala mentén áll: függőlegesen a padlóban vagy a mennyezetben, vízszintesen a falban. Barlang ugyanígy készül: üregelj ki egy téglatestet (Hollow selection) egy rácslépésnyi falvastagsággal, jelöld ki a falakat, és bontsd fel őket.

Az oszlopok alakját a(z) **Formázás** ablak adja meg, amelyet a formázási sáv végén lévő gomb nyit meg; a kijelölést is fel tudja osztani. A **Háromszög** rácscellánként két oszlopot készít, és a háromszög teteje sík marad, akárhogy formázod. A **Négyzet** cellánként egyet készít, vagyis feleannyi brush-t; a teteje egy átló mentén hajlik meg. A **Hatszög** két rácslépés széles hatszögekkel fedi le a brusht, mindegyiket hat háromszögre osztja a középpontja körül, így minden tető sík marad.

Válaszd a **Formázás** eszközt (`J`), és húzd végig a felületen a 3D nézetben. A narancssárga gyűrű mutatja az ecsetet. Kijelölés nélkül az ecset minden alatta lévő brush-t módosít, kijelöléssel csak a kijelölteket. Az ablak tetején lévő sáv adja meg, mit tesz egy húzás:

- Az **Emelés** és a **Süllyesztés** kifelé vagy befelé mozgatja a felületet, a gyűrű közepén a legjobban, a szélén egyáltalán nem. A `Shift` lenyomva tartásával felcserélheted őket.
- A **Simítás** elsimítja a dudorokat.
- A **Simítás síkba** arra a szintre simítja a felületet, ahol a húzás kezdődött.

Ugyanebben az ablakban az **Irány** adja meg, merre:

- A **Fel** egyenesen fel és le mozgatja a felületet, egy talajon fekvő körön belül. Ez a terephez való.
- A **Kamera** a nézet felé és attól el mozgatja, a mutató körüli gömbön belül. Nézz egy padlóra, falra vagy mennyezetre, és azt formázza.
- A **Normális** kifelé mozgatja abból a lapból, amely a mutató alatt van ott, ahol a húzás kezdődik.

A két szám a **Sugár (egység)** és az **Erősség (%)**. Csak a húzás által mozgatott oldal sarkai mozognak; a hátoldal a helyén marad, és egy oszlop legalább egy egység vastag marad. Ahol brush-ok osztoznak egy sarkon, ott együtt mozgatják, így a felület zárt marad. Egy húzás egy lépésben visszavonható. A 2D nézetekben az eszköz úgy jelöl ki, mint a Kijelölés eszköz.

Ha végzett a formázással, jelölje ki a terepet, és válassza a Geometria → **Oszlopok egyesítése** parancsot vagy a(z) **Formázás** ablak gombját. Az egymás melletti oszlopok, amelyek együtt konvex brusht alkotnak és ugyanúgy néznek ki, egy brushsá olvadnak, így a sík részekből néhány nagy brush lesz. A felület nem mozdul, és az egyesítés egy lépésben visszavonható.

Voxeljátékokhoz hasonló terephez bontsd **Négyzet** oszlopokra, és a(z) **Formázás** ablakban a(z) **Húzás** sorban válaszd a(z) **Kockák** lehetőséget. Egy húzás ekkor minden oszlop teljes tetejét egész rácslépésekkel mozgatja, az oldalakat függőlegesen hagyja, így a terep lépcsőzetesen emelkedik. Minden mód úgy működik, mint eddig; a kis húzások összeadódnak, amíg egy oszlop egy teljes lépést nem emelkedik. A más alakú oszlopok nem változnak.

A(z) **Szabad** mód nem igényel oszlopokat, bármilyen brushokon működik. Az Emelés a mutató körüli gömbben terepet ad hozzá, a Süllyesztés elvesz, így kiugrások és barlangok is készülhetnek. A Simítás kiegyenlíti, a Simítás síkba pedig arra a szintre hozza, ahol a húzás kezdődött. Minden érintés a rácson lekerekítve újraépíti az érintett brushokat, a gömbön kívül pedig mindent úgy hagy, ahogy volt, az éles éleket is. Az eredmény közönséges brush, így minden más eszköz működik rajta, és a(z) **Oszlopok egyesítése** utána csökkenti a számukat. Sűrűbb rács simább felületet és több brusht ad.

# Anyagok és UV-k
## Anyagok betöltése és alkalmazása
Nyissa meg a Lap lap **Anyagok** szakaszát. A **Mappa betöltése** egy képeket tartalmazó mappát, a **.wad betöltése** egy textúracsomagot ad hozzá; mindkettő a **Gyűjtemények** alatt jelenik meg, ahol a kereszt eltávolítja őket. A lista szűkítéséhez írjon az **Anyagok szűrése** mezőbe. Egy anyagra kattintva az lesz az aktuális, majd az **Alkalmazás a kijelölésre** minden kijelölt lapra felteszi. A **Kijelölés** menü a Brush-ok kijelölése, amelyek ezt használják és a Lapok kijelölése, amelyek ezt használják lehetőséget kínálja. A **Csere** egy anyagot egy másikra cserél az egész térképen vagy csak a kijelölésben.

A **Festés** eszköz (`K`) az aktuális anyagot a mutató alatti lapra teszi: egy kattintás egy lapot fest ki, a dupla kattintás az egész brush-t, a `Shift`-kattintás pedig a lap anyagát veszi fel aktuálisnak, mint egy pipetta.

## Az UV-szerkesztő
A Lap lap **UV** szakasza a kijelölt lap anyagát és körvonalát mutatja. Egérvezérlés a vásznon:
- A görgő a mutató körül nagyít. A jobb vagy a középső gombbal húzva pásztáz. A jobb gombbal duplán kattintva újra bekeretezi a lapot; egy másik lap kijelölésekor ez automatikusan megtörténik.
- A bal gombos húzás az anyagot mozgatja, a lap körvonala a helyén marad. A gyűrű elforgatja a leképezést, a sarokfogantyúk méretezik, a középső fogantyú pedig e módosítások kiindulópontját állítja be.
- Egy csempe szélén `Alt` és bal gombos húzással nyírhat, ugyanígy `Shift` és középső gombos húzással is.
- Minden húzás egy visszavonási lépés, és az `Esc` megszakítja a folyamatban lévő húzást.

A vászon alatt az **Eltolás**, a **Méretezés**, a **Szög**, és ha a játék használja őket, a **Tartalomjelzők**, a **Felületjelzők** és a **Felületi érték** található. A gombok: **Visszaállítás**, **Visszaállítás a világhoz**, **Illesztés**, **Vízsz. tükrözés**, **Függ. tükrözés** és a két 90°-os forgatás. Az **Igazítás** az anyagot a lap egyik éléhez fordítja (ismételt megnyomás a következő élhez, `Shift` az előzőhöz). A négy igazítógomb az anyagot egy élhez tolja. Az **U illesztése** és a **V illesztése** úgy méretez, hogy egész számú másolat fedje le a lapot; ismételt megnyomással a következő darabszámra vált, a `Ctrl` lenyomva tartásával pedig megmarad a trim sheet felosztása.

## UV-billentyűk
Kijelölt lapoknál a nyílbillentyűk az anyagot mozgatják, a `[` és a `]` 15°-kal elforgatja, a `,` és a `.` felezi vagy duplázza a méretét, a `Ctrl+F` és a `Ctrl+Alt+F` tükrözi, a `Shift+R` visszaállítja, az `Alt+Shift+R` a világ tengelyeihez állítja vissza, a `Ctrl+Shift+F` pedig minden laphoz illeszti.

Az eszközsáv **Zárolás** kapcsolója megtartja a leképezést, amíg a brush-ok mozognak, az **UV** (`U`) pedig, amíg forognak.

## Eszközanyagok
- skip: eltávolítja a megjelölt lapot a motorexportokból.
- clip: látható geometria nélküli ütközést hoz létre.
- origin: egy brush-entitás forgáspontját jelöli.

A nevek az Exportálás ablak **Eszközanyagok** kártyáján módosíthatók. A skip anyagot olyan lapokra használja, amelyekről tudja, hogy rejtve maradnak.

## Lapok kihagyása az exportból
Jelöljön ki egy lapot, és az UV szakaszban kapcsolja be a **Lap kihagyása az exportból** beállítást. Az UV-előnézetben narancssárga sraffozás jelöli. Az anyaga és a festése mentve marad, a brush pedig zárt és szerkeszthető marad. Az exportált renderhálókból ez a lap kimarad, az ütközés viszont megmarad. Ha egy kocka egyetlen négyszögét hagyja meg, két renderháromszög keletkezik. Ez nem teszi a lapokat kétoldalúvá, és nem egyesíti a hálókat.

A **Takart lapok kihagyása** megjelöli azokat a lapokat, amelyeket statikus brush-ok teljesen eltakarnak, beleértve az egymáshoz érő azonos lapokat és a bezárt brush-okat is. A kijelölt brush-okon működik, vagy ha nincs kijelölés, az összes látható, nem zárolt világgeometrián. A brush-entitások, az átlátszó anyagok és a már kihagyott lapokkal rendelkező brush-ok kimaradnak. A részben takart lapokat nem osztja fel. Egyetlen visszavonás mindent visszaállít. A Nézet → Kihagyott lapok megjelenítése elrejti vagy megjeleníti ezeket a lapokat a szerkesztő nézeteiben; a kiválasztás továbbra is az egész brush-ra vonatkozik.

# Szín és színátmenetek
A Lap lap **Szín** szakasza egyszínű festéssel vagy színátmenettel festi ki a lapokat az anyaguk fölött. Hasznos pályavázlatokhoz és stilizált grafikához. A kifestett lapok a nézetekben megtartják valódi színüket; a kijelölést a körvonal mutatja.

## Paletta és keverékszín
A tizenkét palettaszín egyikére kattintva kifestheti vele a kijelölt lapokat vagy a kijelölt brush-okat egészben. Az **Átlátszatlanság** a következő festés fedését adja meg százalékban. A **Keverés** színgombja megnyitja a színválasztót; ha a választóban húz, a kijelölés élőben követi, a **Festés** gomb pedig egy lépésben alkalmazza a keverékszínt. A **Vissza az anyaghoz** eltávolítja a festést, így a lapon ismét az anyaga látszik. A szakasz tetején lévő sor megmondja, mi van kijelölve, és milyen festést visel.

## A színválasztó
A szerkesztő minden színgombja ugyanazt a színválasztót nyitja meg. A négyzetben húzva a telítettséget és a fényességet, a mellette lévő sávban az árnyalatot választhatja ki. Alatta az **R**, **G** és **B** értékek, valamint ahol a színnek van ilyen, az **Átlátszatlanság** csúszka található. A hexa mező a `#RRGGBB` vagy `#RRGGBBAA` formát fogadja el; pirosra vált, amíg a szöveg nem érvényes szín. A **Másolás** a vágólapra teszi a hexa kódot. A választón kívülre kattintva bezárja.

## Színátmenetek
A **Színátmenet** a Lineáris színátmenet vagy a Sugaras színátmenet közül választ, a mellette lévő gomb pedig megfordítja a színek sorrendjét. Minden színpontnak van színgombja és **Pozíció (%)** értéke, a végpontok közötti színpontoknak pedig eltávolító gombja is. A **Színpont hozzáadása** a legnagyobb résbe tesz új színpontot; egy színátmenetnek legfeljebb nyolc színpontja lehet. A **Színátmenet szöge** a lineáris színátmenet irányát állítja be, a **+45°** és **−45°** gombokkal.

Az előnézet alatt egy sín található, minden színponthoz egy jelölővel. A sínre kattintva színpontot ad hozzá, egy jelölőre kattintva megnyitja a színválasztóját, egy jelölőt húzva mozgatja, jobb gombbal kattintva pedig eltávolítja. A sín alatti három minta előbeállítás: Meleg színátmenet, Hideg színátmenet és Szürkeárnyalatos színátmenet. A színátmenet módosításai élőben érvényesülnek a kijelölésen.

A színátmenet iránya fontos: egy tető lejtése vagy egy ívelt objektum mentén tájolja, ne egymással össze nem függő lapokon keresztül. Festés után mindkét oldalról ellenőrizze a modellt.

## Felület anyaga
A **Felület anyaga** az **Érdesség** (0 fényes, 100 matt), a **Fémes** (0 fához, kőhöz vagy műanyaghoz, 100 fémhez) és a **Kibocsátás** (0–8; a felület a saját színében világít, a színátmeneteket is beleértve, de a közeli tárgyakat nem világítja meg) értékét állítja be. A módosítások élőben érvényesülnek a kijelölt lapokon vagy brush-okon. Az **Alapértelmezett felület használata** eltávolítja a felülbírálást. Minden módosítás visszavonható.

A felületi hatások a megvilágított 3D nézetben és a kamerás renderekben látszanak. A sík és a drótvázas megjelenítés szándékosan elhagyja a csillanásokat. A ragyogás fényudvart adhat a kibocsátó felületek köré. A Godot és a glTF/GLB megkapja az anyagértékeket, a Unity importáló pedig az érdességet simasággá alakítja. Az egyes célok korlátait lásd: `docs/surface-materials.md`.

# Eszközök, rétegek és csoportok
## Az eszközkönyvtár
Nyissa meg az **Eszközök** lapot. A **Mappa hozzáadása** gombbal `.map` modelleket tartalmazó mappát adhat hozzá; a beépített könyvtár mindig elérhető. Kereséshez írjon az **Eszközök szűrése** mezőbe. Egy modellre kattintva kijelöli, majd dupla kattintással, az `Enter` billentyűvel vagy a **Beszúrás** gombbal oda szúrja be, ahol a mutató legutóbb volt. Egy modellt egy nézetbe is húzhat. Az eszközök brush-ai egy csoportként kerülnek be, így a modell egyben mozog.

A **Kijelölés mentése eszközként** a kijelölt brush-okat `.map` fájlként írja a könyvtárba. A könyvtár eszközei közönséges, szerkeszthető brush-ok, nem importált hálók.

## Rétegek
A Térkép lap **Rétegek** listája a nagy területek rendszerezésére szolgál. Írjon be egy nevet, és kattintson a **Hozzáadás** gombra új réteg létrehozásához, vagy nyomja meg az `Enter` billentyűt az aktuális réteg átnevezéséhez. Minden réteg elrejthető, zárolható vagy kihagyható az exportokból. A **Mozgatás ide** az aktuális rétegre helyezi a kijelölést. A nézetek helyi menüje az Áthelyezés rétegre, az Aktív réteggé tétel, a Rétegek elrejtése és a Rétegek elkülönítése lehetőséget is kínálja.

## Csoportok és csatolt csoportok
A `Ctrl+G` csoportosítja a kijelölést, a `Ctrl+Shift+G` bontja a csoportot. A Térkép lap **Csoportok** szakasza felsorolja a csoportokat, és a **Megnyitás**, **Átnevezés**, **Egyesítés**, **Csoport bontása**, **Kivétel** és **Leválasztás** műveletet kínálja. A csatolt csoport olyan másolatok halmaza, amelyek osztoznak a módosításokon: a `Ctrl+Shift+D` csatolt másolatot készít, a Geometria → Csatolt csoportok szinkronizálása frissíti a másolatokat, a **Csatolás megszüntetése** pedig függetlenné tesz egy másolatot. Az Entitás lapon egy tulajdonság megtartható egy másolat sajátjaként a csatolt csoportok szinkronizálásakor.

## Közös lépték
Az alapértelmezett exportlépték 32 térképegység méterenként. A beépített játékoskezdőpont-jelölő 64 egység magas, ez 2 méteres viszonyítási alap. Ehhez mérje az ajtónyílásokat, járműveket, bútorokat és növényeket. Az exportléptéket csak szándékosan módosítsa, és tartsa összhangban az összetartozó eszközöket.

# Entitások és játékmenet
A gyakori entitásdefiníciók beépítettek és mindig elérhetők: fények, játékoskezdőpontok, ajtók, gombok, platformok, eseményindítók, statikus kellékek, hangok és útpontok. Ha az Entitás lapon az **Entitásdefiníciók** segítségével betölt egy projekt-FGD-t, az lecseréli az azonos nevű osztályokat, a többi beépített osztály pedig megmarad.

## Entitások elhelyezése
Az Entitás lap **Entitások** böngészőjében írjon az **Osztályok szűrése** mezőbe, és válasszon egy osztályt. Pontosztálynál, például fénynél, kattintson az **Elhelyezés** gombra, vagy kattintson duplán az osztályra; az `Insert` egyet a kijelölés középpontjába helyez. Brush-osztálynál, például ajtónál vagy eseményindítónál, jelölje ki a brush-okat, és kattintson a **Létrehozás** gombra. Az osztályokat egy nézetbe is húzhatja.

## Tulajdonságok
A tulajdonságlista a kijelölt entitást szerkeszti. Az azonos osztályú entitások együtt szerkeszthetők. A **Tulajdonság hozzáadása** olyan kulcsot is hozzáad, amelyet a definíció esetleg nem ismer; az **Alapértékek** a meglévő tulajdonságokat visszaállíthatja az alapértékükre, vagy hozzáadhatja a hiányzókat. Az osztály által definiált, de az entitásnál hiányzó tulajdonságok halványan jelennek meg; érték beírásával hozzáadódnak. Az entitások összekapcsolásához használja a targetname és a target tulajdonságot.

## Mire képes egy export
A Godot direktívái natív csomópontokat, ütközőtesteket és eseményindító területeket hoznak létre. A Unity és az Unreal támogatja a gyakori natív fényeket, az Unreal ezenfelül PlayerStart actorokat is létrehoz. A többi adat annyira marad meg, amennyire a cél importálója engedi.

Egy ajtóentitás önmagában nem ad játékbeli viselkedést. A mozgást, az eseményindítókra adott válaszokat, a játékosvezérlőket és a projekt szkriptjeit továbbra is a motorban kell elkészíteni. A Defold az entitások tulajdonságait és ütközőobjektumait exportálja; a Flax modelleket és általános entitásokat helyez el. Mielőtt játékmenetet építene rá, ellenőrizze az eredményt a motorjában.

# Animáció
A Heron Brush képes egyszerű mozgások előnézetére és a brush-entitások kulcskockás animációinak szerkesztésére, majd ezeket több motorba is exportálja.

## Animációk előnézete
Kapcsolja be az **Animációk előnézete** beállítást a Nézet menüben, az Animáció menüben, a Rácsbeállítások menüben vagy a Térkép lap Nézet szakaszában. Lejátssza a func_rotate forgását és a func_shuttle oda-vissza mozgását, ha a start_on be van kapcsolva, az entitás tengely-, sebesség-, irány-, út- és várakozási tulajdonságai alapján, valamint lejátssza az indításkori lejátszásra beállított kulcskockás klipeket. A ventilátor és a szélmalom eszköz már tartalmaz forgó részeket.

Az előnézet kikapcsolása visszaállítja az eredeti helyzetet. Egy nézetbe kattintva a lejátszás leáll a kijelölés vagy szerkesztés előtt; a kameranavigáció továbbra is működik. Az előnézet nem módosítja a térképet, a visszavonási előzményeket vagy az exportokat, és nem futtat fizikát vagy játéklogikát.

## A kulcskocka-szerkesztő
Az Animáció → Kulcskockás animáció menüponttal vagy a Rácsbeállítások menüből nyitható meg. A nézetek alá dokkolódik, vagy a 3D ablakba, amíg az külön van. A fejlécében lévő ablak ikon saját ablakba helyezi át; a képkocka és a visszavonási előzmények megmaradnak. Az **Animáció szerkesztése** a nézeteket az animált helyzetre váltja, a **Térkép szerkesztése** pedig visszatér a nyugalmi geometriához. Animációs módban minden nézet alján az **Animáció szerkesztése** felirat és a képkocka száma látható.

Animáláshoz jelöljön ki egy brush-entitást, egy brush-t vagy egy brush-csoportot. Egy egyszintű brush-csoport egy animáción és forgásponton osztozik; a több entitást tartalmazó csoportokhoz külön animációk kellenek.

## Klipek
Az **Animáció** választó a névvel ellátott klipek között vált. Írjon be egy nevet az **Animáció neve** mezőbe, és kattintson a **Mentés másként** gombra az aktuális klip ilyen néven történő másolásához; a **Törlés** eltávolít egy klipet. Az **Eltolás** forgatás nélküli `translate` klipet hoz létre. Az **Ajtóanimációk** sima `open` és `close` klipeket hoz létre 90 fokos fordulattal az aktuális forgáspont körül; előbb helyezze a forgáspontot a zsanérhoz. A **Lejátszás indításkor** a pálya indulásakor lejátssza a klipet; az ajtó-előbeállítás kikapcsolja.

## Vezérlés és lejátszás
A vezérlőgombok: **Kezdet**, **Előző kulcs**, **Lejátszás**, **Következő kulcs** és **Vége**. A **Képkocka** mutatja és beállítja az aktuális képkockát, az **Időtartam (mp)** pedig a klip hosszát. A mód **Egyszer**, **Ismétlés** vagy **Oda-vissza**.

## Az idővonal
Az idővonalon egy vonalzó található a lejátszófejjel, valamint egy sáv a **Pozíció** számára (X, Y és Z együtt kulcsolva) és mindegyik forgásszöghöz, RX, RY és RZ. A vonalzóra kattintva vagy húzva mozgathatja a lejátszófejet, és minden nézetben megtekintheti az adott helyzetet. A középső gombbal húzva pásztázhatja az idővonalat. A kulcsokat húzással mozgathatja az időben; a `Shift` hozzáad a kijelöléshez; egy sávra duplán kattintva kulcsot szúr be; a `Ctrl`+görgő nagyít. A **Görbék** a sávok helyett az értékgörbéket mutatja.

A fölötte lévő gombok: **Kulcs hozzáadása**, **Eltávolítás**, **Másolás**, **Beillesztés** és **Kettőzés**. Az **Illesztés képkockákhoz** egész képkockákon tartja a kulcsokat, az **Illesztés** pedig a teljes klipet mutatja. A **Kulcsok időzítésének módosítása** megnyitja az **Időskála**, az **Eltolás (képkocka)**, a **Méretezés a lejátszófejtől**, az **Időtartam meghosszabbítása** és a **Kijelölés időzítésének módosítása** beállítást.

Az idővonal melletti kulcsvizsgáló a kijelölt kulcsot szerkeszti: **Képkocka**, **Érték** (vagy Pozíció-kulcs esetén a három koordináta), a **Lineáris**, **Konstans** vagy **Bézier** interpoláció, **Bejövő meredekség** és **Kimenő meredekség**, valamint **Sima érintők**.

## Billentyűk a szerkesztőben
Amíg az idővonalé a billentyűzet: a `Space` lejátszik vagy szüneteltet, a bal és jobb nyíl egy képkockát lép, a `Ctrl+A` minden kulcsot kijelöl, a `Ctrl+C`, `Ctrl+X` és `Ctrl+V` kulcsokat másol, vág ki és illeszt be, a `Ctrl+D` kettőzi őket, a `Delete` eltávolítja őket, a `Ctrl+Z` visszavon, az `Esc` pedig megszakítja a húzást. Az idővonalon kívül az `Esc` kilép az animációs módból. A visszavonás a helyén hagyja a nagyítást, a kijelölést és a lejátszófejet.

## Beállítás a nézetekben
Az idővonal alatti sorban található a **Hagymahéj-nézet** darabszámmal és **Térköz** értékkel, amely a közeli helyzeteket szellemképként mutatja; a **Forgáspont** koordinátái; az **Automatikus kulcs**; valamint az **Eltolás**, **Forgatás** és **Forgáspont** helyzetbeállító eszközök.

- A **Kulcs hozzáadása** bekapcsolja az Automatikus kulcsot, és szünetelteti a lejátszást. Bekapcsolt Automatikus kulccsal és az **Eltolás** kiválasztásával a 3D nézetben a kijelölt entitás fogantyújának egy színes tengelyét húzva kulcsolhatja a pozícióját az aktuális képkockán.
- Egy 2D nézetben a kijelölt brush-t vagy a csoportja egy tagját húzva a két látható tengely mentén kulcsolhatja az eltolást, rácsillesztéssel. Ehhez nem kell bekapcsolni az Automatikus kulcsot.
- A **Forgatás** eszközzel egy 2D nézetben a brush-t a narancssárga forgáspontjelölője körül húzhatja. A Felül nézet RZ, az Elöl nézet RY, az Oldalt nézet RX kulcsot hoz létre, a rács szögeihez illesztve.
- A **Forgáspont** eszközzel bármelyik 2D nézetben a sárga forgásközéppontot húzhatja. A rácshoz illeszkedik, és megtartja a mélységét. A kulcsok és a nyugalmi geometria változatlanok maradnak.

A 2D és 3D nézetben látható ciánkék útvonal a forgáspont mozgását mutatja. Egy útvonalpontra kattintva kijelöli annak idejét és kulcsait, húzásával pedig módosítja a pozíciót; bekapcsolt Automatikus kulccsal a 3D nézetben is húzhatja az útvonalpontokat. A címkék a képkockaszámokat mutatják, a szaggatott szakaszok pedig ugrásokat jelölnek. Az `Esc` ezek bármelyikének húzását megszakítja, és egyetlen visszavonás egy teljes húzást visszafordít.

Az új animációk nyugalmi helyzetből indulnak, kulcsokkal a nulla időpontban. Mozgást úgy hozhat létre, hogy egy másik képkockán módosítja a helyzetet. Éles, L alakú útvonalhoz kulcsolja a kezdetet, a sarkot és a véget, és használja a Lineáris interpolációt az Oda-vissza móddal.

## Gombok összekapcsolása animációkkal
Jelöljön ki egy func_button, trigger_multiple vagy trigger_once entitást. Az Entitás lap a tulajdonságok fölött az **Animációs eseményindító** részt mutatja. Válasszon célt a listából, vagy kattintson a **Cél kijelölése** gombra, majd egy animált objektumra bármelyik nézetben; az `Esc` megszakítja a kiválasztást. A hiányzó célokat vagy klipeket a vezérlők alatt jelzi. Az entitáskapcsolatok nyilakat mutatnak a céljaikhoz, ha a kapcsolatok megjelenítése be van kapcsolva.

A **Művelet** azt választja ki, mit tegyen az aktiválás: a **Lejátszás** elindít egy névvel ellátott klipet vagy a kijelölt animációt; a **Váltás** felváltva játssza le az `open` és a `close` klipet, ha léteznek, különben újraindítja a kijelölt animációt; a **Megnyitás** és a **Bezárás** ezeket a klipeket indítja; a **Szünet** megtartja a helyzetet, a **Folytatás** folytatja, a **Leállítás** pedig visszaállítja az eredeti helyzetet. A **Csak egyszer** felülbírálja az osztály alapértelmezését. Az **Ismétlési késleltetés (mp)** az aktiválások közötti legrövidebb időt adja meg; az elsőt nem késlelteti.

Az **Eseményindító kipróbálása** az előnézetben futtatja a műveletet anélkül, hogy a térképet szerkesztené vagy visszavonási lépést adna hozzá. Az **Előnézet visszaállítása** visszaállítja az eredeti helyzeteket, és törli az egyszeri és a várakozási állapotot. Ez nem szimulálja a játékos ütközéseit vagy a játék szkriptjeit.

## Animáció az exportokban
A Godot natív AnimationPlayer és AnimationLibrary erőforrásokat exportál, a görbékkel és a mozgó ütközéssel együtt. Ha egy CharacterBody3D belép egy gomb területére, aktiválja annak célját. Az ezeken a klipeken Godotban végzett módosítások nem kerülnek vissza a térképbe, és a következő export felülírja őket.

A raylib export hozzáadja a `<map>_animation.h` fájlt egy C lejátszóval és a `<map>_preview.c` fájlt egy szabad kamerás előnézettel; a fordításhoz kövesse a `<map>_raylib.md` útmutatót. Az előnézetben a `SPACE` szüneteltet, az `R` újraindítja az alapértelmezett klipeket, az `N` pedig végiglépked a névvel ellátott klipeken. A Defold az entitás szkriptjébe ágyazza a lejátszást, dokumentált `heron_*_animation` üzenetekkel. A Unity Build Level parancsa hozzáadja a `HeronBrushAnimation` komponenst és egy kinematikus testet. A Bevy export tartalmazza a `heron_animation.rs` fájlt és az útmutatót. A Godot, a Unity, a Defold, a Bevy és a raylib megtartja az eseményindító-kapcsolatokat, az egyszeri jelzőt és az ismétlési késleltetést. A játékának továbbra is regisztrálnia kell a lejátszót, és a saját gombjait össze kell kötnie az egyes futtatókörnyezetek API-jával.

## A Motion lab példa
Nyissa meg a `motion_lab` példát a kezdőképernyő Példák részéből. Tartalmaz egy L alakú útvonalon mozgó platformot, egy liftet, egy forgó platformot és egy zsanéros ajtót gombokkal. Nyissa meg a mellette lévő `godot/project.godot` fájlt Godotban, és nyomja meg az `F5` billentyűt. A WASD és az egér mozgatja a játékost, a Space ugrik, az 1–4 billentyűk az állomásokra visznek, az R pedig újraindítja az aktuális állomást. Lépjen egy színes lapra a célja aktiválásához. A mellékelt README leírja az újraexportálást.

# Exportálás motorba
A Fájl → Exportálás megnyitja az Exportálás ablakot. A motorok bal oldalon szerepelnek a logóikkal: Godot, Defold, three.js, Bevy, raylib, Unity, Unreal Engine, Flax Engine, Cocos Creator, Wavefront .obj, glTF .gltf, GLB .glb, végül a `.map`. A jobb oldali oldal leírja, mit kap a cél. A **Kiírás** kiírja, az **Ellenőrzés** kiírás nélkül felépíti a pályát, és felsorolja a problémákat, a **Bezárás** pedig bezárja az ablakot.

Az exportálás független a mentéstől: a `.map` marad a szerkeszthető forrás. Minden vezérlő azonnal elmenti a motorja beállításait, amint megváltozik, így minden motor megőrzi a saját válaszait, és az ablak úgy nyílik meg, ahogy hagyta.

## Gyakori beállítások
- **Méretezés**: **Egység méterenként**, alapértelmezés szerint 32.
- **Mit írjon ki**: **Ütközési geometria**.
- **Optimalizálások**: **Statikus mesh-ek csoportosítása 1024 egységes cellákba**, **A geometria belsejébe temetett lapok elhagyása**, **Az egy síkban lévő lapok egyesítése** és **Ütközési alakzatok egyesítése**.
- **Projektútvonalak**: hová kerül a pálya és az erőforrásai a projekten belül.
- **Eszközanyagok**: a clip, skip és origin anyagok neve.
- **Továbbiak**: motorspecifikus kapcsolók, például **Csomópontok elnevezése a következő szerint**, **Rétegek és csoportok újraépítése szülőcsomópontként**, **A háromszögek körüljárási irányának megfordítása**, a textúrakoordináták tükrözése és egy második UV-készlet a sütött fényhez a **Lightmap-texel** beállítással.
- **Lépéstartás a térképpel**: **Újraírás a térkép minden mentésekor**. Akkor válik elérhetővé, ha a motorba már egyszer exportált, mert az az export határozza meg, hová kell írni.

A statikus cellák a világ brush-ait a középpontjuk alapján sorolják be, és minden 1024 egységes cellán belül anyagonként egyesítik őket, így a motorok a térkép kisebb részeit is ki tudják vágni a megjelenítésből. Az ütközés megmarad, a játékmenet-entitások pedig külön maradnak. Ez több rajzolási hívást jelenthet, mint egyetlen világháló, ezért alapértelmezés szerint ki van kapcsolva. Az önálló OBJ, glTF és GLB exportok nem használnak cellákat.

## Megjegyzések a célokhoz
- Godot: a textúrák és anyagok `res://` útvonalai, az eseményindítók fizikai rétege és a lightmap UV2. Válasszon exportmappát a Godot projekten belül.
- Defold: az eszközök erőforráskönyvtára, a modell anyaga és a textúra-mintavételező, amelynek léteznie kell a kiválasztott anyagban.
- Unity: OBJ hálók és egy szerkesztőbeli importáló; a projekt Assets mappája alá exportáljon.
- Unreal Engine: egy generált importszkript és egy tartalomútvonal, általában /Game/<level>.
- Flax Engine: modell-tartalomútvonal a projekt Content mappájához képest.
- three.js, Bevy, raylib és Cocos Creator: jelenetek vagy kód ezekhez a futtatókörnyezetekhez, az export mellett útmutatóval.
- glTF / GLB: hordozható geometria a kompatibilis eszközökhöz; a GLB a bináris változat. OBJ: geometria és anyagok modellező eszközökhöz.
- `.map`: a **Kiírás mint** a Valve 220, Normál, Quake 2 vagy Quake 2 (Valve) változatot választja. Az ablak megmondja, mit veszítene a választott dialektus.

Kiírás után olvassa el a figyelmeztetéseket, például a hiányzó textúrákról. Az ablakban **Az ellenőrzés eredménye** sorolja fel őket.

# Élő kapcsolat: Godot, Unity, Defold és Flax
Az élő kapcsolat munka közben elküldi a Heron Brush nem mentett módosításait egy motorszerkesztőbe. Egyirányú: a motorban végzett módosítások nem kerülnek vissza. Térképablakonként egy élő kapcsolat futhat. Motorváltás előtt állítsa le, a módosított exportbeállítások érvényesítéséhez pedig indítsa újra. A vezérlők az Exportálás ablakban, a motor oldalán, az **Élő kapcsolat: …** kártyán találhatók.

A címsorban lévő jelző a kapcsolat állapotát mutatja; rákattintva leállíthatja a kapcsolatot. A térkép bezárása vagy lecserélése leállítja a közzétételt. Importálási hiba után javítsa ki a problémát, és indítsa újra a kapcsolatot.

## Godot
1. Exportáljon egyszer egy Godot projektbe.
2. Kattintson a **Godot-bővítmény telepítése** gombra.
3. Engedélyezze a Heron Brush Live bővítményt a Godot Project Settings → Plugins részében.
4. Kattintson az **Élő kapcsolat indítása** gombra, és a Godot dokkjából csatolja a térképet egy megnyitott 3D jelenethez.

A bővítmény csak a saját generált ágát cseréli le, és megtartja a csatolás elhelyezését és a többi csomópontot. A Godot jelenetet saját maga mentse, és a saját szkriptjeit és csomópontjait tartsa a generált ágon kívül.

## Unity
1. Exportáljon egyszer a Unity projekt Assets mappája alá.
2. Kattintson a **Unity-bővítmény telepítése** gombra, és hagyja, hogy a Unity lefordítsa.
3. Nyissa meg Unityben a Window → Heron Brush → Live Link menüpontot.
4. Indítsa el a kapcsolatot, és csatolja a térképet az aktív Unity jelenethez.

Tartsa nyitva a Unity élő ablakát. Play Mode és Prefab Mode alatt a frissítések szünetelnek. Csak a térképobjektum Generated gyermekobjektuma cserélődik le. Ne szerkessze a generált fájlokat.

## Defold
1. Exportáljon egyszer a Defold projektjébe, és tartsa nyitva a projektet a Defoldban.
2. Kattintson az **Élő kapcsolat indítása** gombra. Nincs szükség bővítményre.
3. Nyissa meg Defoldban a heron_brush_live/preview.collection fájlt, vagy hivatkozzon rá a saját jelenetéből.

A kapcsolat egy külön generált mappába ír, a végleges exportot és a game.project fájlt érintetlenül hagyja. Ha bekapcsolja **A futó játék gyors újratöltése** beállítást, a Defoldból indított játék is frissül; objektumok hozzáadásához vagy az ütközés módosításához a pálya újraindítására lehet szükség. A sikertelen kapcsolódásokat automatikusan újra megpróbálja.

## Flax
Exportáljon a projekt Content mappájába, és kattintson a **Flax-bővítmény telepítése** gombra. Hagyja, hogy a Flax lefordítsa, indítsa el a kapcsolatot, majd a megnyitott Flax jelenetben válassza a Tools → Heron Brush: attach live map menüpontot. A generált gyökér megtartja az elhelyezését, a többi objektum érintetlen marad. A jelenetet saját maga mentse. Play Mode alatt a frissítések szünetelnek. A Tools → Heron Brush: stop live map szünetelteti a fogadást.

# Közös élő munkamenetek
Az Együttműködés → Élő munkamenet egy ablakot nyit, amelyben munkamenetet indíthat egy másik Heron Brush-felhasználónak, vagy csatlakozhat hozzá. Ez a térkép szerkesztését osztja meg emberek között; független a motorral való élő kapcsolattól.

## Indítás és csatlakozás
Adja meg **Az Ön neve** értékét. Munkamenet indításához válassza a **Munkamenet indítása** lehetőséget, kattintson a **Munkamenet indítása** gombra, és küldje el a vendégeinek a **Kapcsolati adatok másolása** gombbal kimásolt adatokat. Csatlakozáshoz válassza a **Csatlakozás munkamenethez** lehetőséget, illessze be az adatokat, és kattintson a **Csatlakozás** gombra. A csatlakozás a megnyitott térképet a gazda térképére cseréli, ezért előbb mentse a munkáját.

A normál kiadás a helyi hálózaton működik, a portot a **Helyi hálózati port** adja meg. Az interneten keresztüli munkamenetekhez (**Csatlakozás az interneten keresztül**) az opcionális hálózati funkcióval készült kiadás szükséges. A résztvevők a 3D nézetben látják egymás kamerajelölőit és nevét.

Az ablak bezárása után a munkamenet tovább fut; az állapotát a címsor jelzője mutatja. A **Munkamenet leállítása** vagy a **Kilépés a munkamenetből** befejezi. Ha ketten ugyanazt az objektumot szerkesztik, a gazda sorrendje dönti el a végleges változatot, és a visszavonás is közös módosításnak számít. Ahol lehet, külön területeken dolgozzanak, és mentsenek ellenőrzőpontokat.

# 3D megjelenítés és második monitor
## Megjelenítési módok
A Térkép lap **Nézet** szakasza azt kapcsolja, mit rajzoljanak a nézetek: Brush-ok, Pontentitások, Entitásmodellek, Entitások osztálynevei, Csoportok körvonala, Patch-ek, Anyagok, Kihagyott lapok megjelenítése, Animációk előnézete, Drótváz, Élek és Brush-entitások körvonala. A Drótváz a háromszögek éleit rajzolja meg, az átlókat és a felületek mögötti éleket is, anyagok nélkül. Ez ideiglenes hibakereső nézet, és nem befolyásolja az exportokat vagy a rendereket.

A 3D nézet megvilágítása, árnyékai, környezeti takarása, tükröződései, ragyogása és simítása a Beállítások → Megjelenítő oldalon található. Ezek előnézeti beállítások; a végleges képek a renderkamera saját beállításait használják.

## Külön 3D ablak
A Nézet → 3D nézet külön ablakban saját ablakba helyezi a 3D nézetet, amelyet másik monitorra is áttehet. Ugyanazt a kamerát, dokumentumot és eszközöket használja, a főablakban pedig a sík nézetek foglalják el a helyet. A menüje az Anyagok, a Tömör nézet és a Drótváz lehetőséget kínálja; egy Nézet almenüt a Kihagyott lapok megjelenítése, Animációk előnézete, Élek, Csoportok körvonala, Pontentitások, Entitásmodellek és Brush-entitások körvonala elemekkel; a Fókusz a kijelölésre; az Elöl, Oldalt és Felül pontot, amelyek a kamerát a kijelölés (vagy egy a kamera előtti pont) köré fordítják; a Kamera elhelyezése a 3D nézetből; a Teljes képernyő (`F11`); a Kép renderelése; és a Vissza a főablakhoz pontot. Az ablak bezárása visszadokkolja a nézetet.

## Procedurális égbolt
A procedurális égbolt a 3D nézetben alapértelmezés szerint be van kapcsolva, és a Beállítások → Megjelenítő → Fény oldalon kapcsolható ki. A renderkameráknak saját **Procedurális égbolt** kapcsolójuk van, amely a térképpel együtt mentődik. Az égbolt, a horizont és a talaj színe a környezeti beállításokból származik, a nap pedig a jelenet megvilágítását követi. Az égboltnak nincs geometriája, és nem exportálódik.

# Renderkamerák és PNG-képek
A Fájl → Kép renderelése megnyitja a renderablakot: bal oldalon a kamera és a beállításai, jobb oldalon egy élő előnézet, amely a módosításokkor újrarajzolódik. A kamerák a `.map` fájlban, egy csak a szerkesztőben létező Render cameras rétegben mentődnek, amely kimarad a motorexportokból.

## A kamera elhelyezése
Állítsa be a képkivágást a 3D nézetben, és nyomja meg a `Ctrl+Alt+0` (vagy `Ctrl+Alt+Num0`) billentyűket, vagy kattintson a **Kamera elhelyezése a 3D nézetből** gombra. Ha nincs kijelölt kamera, az első mentett kamera frissül; ha egy sincs, új jön létre. Ez visszavonható, és megtartja a kimeneti és megvilágítási beállításokat. A **Kamera kijelölése a térképen**, a **Kamera betöltése a térképből** és a **3D nézet mozgatása a kamerához** az ellenkező irányban működik. A **Kamerakeret a 3D nézetben** a kimenet határát mutatja, a **Kamera vezérlése a 3D nézetből** pedig lehetővé teszi, hogy a szokásos navigáció a mentett kamerát mozgassa.

## Beállítások
- **Kamera pozíciója**: X, Y, Z, elfordulás és dőlés.
- **Kimenet és objektív**: **Előbeállítások** (Full HD, Négyzet, Álló), szélesség, magasság, mintaszám, **Függőleges látószög** és **Távoli vágósík**.
- **Megvilágítás**: Nappali fény, Naplemente vagy Borult, illetve az expozíció, a nap, az égbolt, a horizont és a talaj színe, **Procedurális égbolt**, **Árnyékok**, **Környezeti takarás** és **Ragyogás**.
- **Mélységélesség**: **Mélységélesség bekapcsolása**, fókusztávolság, éles tartomány és elmosási sugár. A **Fókusz a kijelölésre** a kamera előtt lévő kijelölt objektum középpontjára fókuszál.

A **Kamera és beállítások mentése** a térképbe menti az űrlapot; a lemezen való megőrzéshez mentse a térképet.

## Renderelés
A **PNG renderelése** megkérdezi, hová mentse, és fokozatos renderelést indít, amely mintáról mintára töltődik ki az előnézetben. Több minta szebb eredményt ad, de tovább tart. A **Renderelés megszakítása** fájl kiírása nélkül leáll. A renderelő a szerkesztő raszteres folyamatát használja, nem sugárkövetést. A PNG átlátszatlan hátterű, és az animációk nem renderelődnek. Nagyon nagy képek rövid időre megakaszthatják a szerkesztőt.

# Beállítások
A Fájl → Beállítások (`Ctrl+,`) megnyitja a beállításokat. A módosítások azonnal érvényesülnek és mentődnek. Az oldalsáv az Áttekintés, Játékok, Nézet, Színek, Egér, Billentyűzet és Megjelenítő oldalt sorolja fel. A felül lévő keresőmező bármelyik oldalon név vagy leírás alapján megtalálja a beállítást. Minden módosított beállításnak saját gombja van az alapértéke visszaállításához, az **Alapértékek visszaállítása** pedig az aktuális oldalt állítja vissza.

## Az oldalak
- **Áttekintés**: a gyakori beállítások együtt: Megjelenés (Panel átlátszatlansága (%), Színséma, Elrendezés, Nyelv), Szerkesztés, Nézet és Kézikönyv.
- **Játékok**: a játék, a fájljainak helye és az együttműködéshez használt neve. A játék határozza meg, milyen térképformátumot írjon a program, milyen entitásdefiníciókat töltsön be, és hol vannak az anyagai. A játékkonfigurációkat a program a konfigurációs mappa `games` mappájából (Linuxon és macOS-en `~/.config/heron-brush/games`, Windowson `%LOCALAPPDATA%\heron-brush\games`) és az alkalmazás melletti `games` mappából olvassa; a csomagok ez utóbbiba teszik a mellékelt konfigurációt.
- **Nézet**: Elrendezés, Színséma, Nyelv, világtengelyek, a rács átlátszatlansága, a szivárgásellenőrzés cellamérete és a leképezési zárak.
- **Színek**: a nézetek, a geometria és a fogantyúk színei.
- **Egér**: a körülnézés, a pásztázás és a mozgatás sebessége és invertálása, repülési sebesség, látószög, 2D nézetek összekapcsolása és a fogantyúk mérete.
- **Billentyűzet**: a billentyűkiosztás, Heron Brush vagy Blender.
- **Megjelenítő**: valósághű árnyalás, kijelölés körvonala, fény, árnyékok, környezeti takarás, tükröződések, ragyogás és felületek.

## Nyelv
A **Nyelv** a Rendszernyelv követése lehetőséget és minden lefordított nyelvet kínál. A felület azonnal átvált. A kézikönyv a felület nyelvén nyílik meg, ha létezik a fordítása, egyébként angolul.

# Bővítmények és MCP
## Bővítmények
A bővítmények Rhai szkriptekkel bővítik a szerkesztőt. A Fájl → Bővítmények megnyitja a Bővítmények ablakot. A **Bővítmény telepítése** egy `.rhai` fájlt, a **Bővítménymappa telepítése** egy `main.rhai` fájlt tartalmazó mappát telepít; az opcionális `plugin.toml` metaadat leírja a bővítményt és a beállításait. A bővítmények az **Aktív bővítmények** és az **Inaktív bővítmények** csoportban jelennek meg, a **Bővítmény engedélyezése** pedig be- vagy kikapcsol egyet; ez újraindítás után is megmarad. A **Bővítmények újratöltése** újra beolvassa a szkripteket a lemezről, és alaphelyzetbe állítja a paneljeik értékeit.

Az aktív bővítmények a Térkép lap **Bővítmények** szakaszában is megjelennek a beállításaikkal, műveletgombjaikkal (**Futtatás**) és kimenetükkel. Egy művelet a kijelölésen dolgozik, vagy ha nincs kijelölés, mindenen, és egy visszavonási lépést jelent. A mellékelt Selection inspector darabszámokat, valamint felfelé és lefelé mozgató műveleteket mutat.

## MCP
Az Együttműködés → Kiszolgálás ügynöknek (MCP) lehetővé teszi, hogy egy MI-ügynök vagy más eszköz a Model Context Protocolon keresztül vezérelje a megnyitott szerkesztőt. Az ablak a helyi címet mutatja a **Másolás** gombbal, a **Kiszolgáló leállítása** pedig leállítja; amíg fut, a címsorban az **MCP aktív** felirat látható. A `--mcp` parancssori kapcsoló ugyanezt a felületet ablak nélkül futtatja. Az ügyfelek felfedezhetik a geometriához, eszközökhöz, tulajdonságokhoz, animációhoz, exportáláshoz és munkamenetekhez tartozó eszközöket. Az MCP és a szerkesztő bővítményei különböző felületek.

# Parancssor
A program neve `heron-brush`. Kapcsolók nélkül a kezdőképernyőt nyitja meg.

## Fájlok megnyitása
A `heron-brush map.map [entities.fgd] [materials/]` megnyit egy térképet, opcionálisan entitásdefiníciókkal és anyagmappával. Egy üres argumentum kihagy egyet, például `heron-brush map.map "" materials/`. Ha egy szerkesztő már fut, a térkép abban nyílik meg; a `--new-instance` ehelyett külön ablakot nyit.

## Parancsok
- `--help` vagy `-h`: kiírja a használati útmutatót. `--version` vagy `-V`: kiírja a verziót.
- `--mcp`: MCP kiszolgálása a szabványos bemeneten és kimeneten, ablak nélkül.
- `--merge-map <base> <ours> <theirs> [<output>]`: `.map` fájlok háromutas egyesítése gittel való használathoz. A kimenet alapértelmezés szerint az ours. Az ütközéseket kiírja, és nem nulla kilépési kódot ad.
- `--obj-to-map <in.obj> <out.map> [size] [thickness] [snap] [material]`: hálót alakít brush-okká. A size a leghosszabb oldal hossza térképegységben (alapértelmezés 256), a snap a sarkok rácsa (0, ha nincs), a `solid` vagy `layered` szó pedig vékony ékek helyett rácsdobozokat vagy konvex lapokat épít.
- `--render-preview <in.map> <out.png> [size]`: PNG-bélyegképet ír egy térképről, alapértelmezés szerint 512 képpont méretben.
- `--run-script <script.rhai> <in.map> [out.map] [key=value ...]`: egy bővítményszkriptet futtat egy térképen, mindent kijelölve, és kiírja az eredményt (alapértelmezés szerint vissza a bemenetbe). A `key=value` a bővítmény beállításaira válaszol.

# Hibaelhárítás
## Ha valami nem jól néz ki
- Hiányzó anyag: ellenőrizze az anyagmappát vagy -csomagot és az export projektútvonalait.
- Rossz modellméret: vesse össze a 64 egységes játékosjelölővel és az exportléptékkel.
- Rossz színátmenet-irány: ellenőrizze a színátmenet szögét és a lap tájolását.
- Üres motorjelenet: olvassa el az export figyelmeztetéseit, és kövesse a cél importálási útmutatóját.
- Az élő kapcsolat várakozik: ellenőrizze, hogy a bővítmény telepítve van-e, a jelenet csatolva van-e, és a frissítések nem szünetelnek-e.
- Csak vonalak, lapok nélkül: kapcsolja ki a Drótvázat a Térkép lap Nézet szakaszában, vagy válassza az Anyagok vagy a Tömör nézet lehetőséget a 3D ablak menüjében.
- Objektumok hiányoznak a nézetekből: a `Ctrl+Shift+I` megjeleníti a rejtett objektumokat; ellenőrizze a rétegeket és a View filters beállításokat.
- Egy billentyű nem csinál semmit: előbb kattintson a nézetbe, hogy az kapja a billentyűzetet, és ellenőrizze a billentyűkiosztást a Beállítások → Billentyűzet oldalon.
- Egy szerkesztés sikertelen: vonja vissza, egyszerűsítse a geometriát, és használja a Térkép lap **Problémák** szakaszában a **Térkép ellenőrzése** és a **Lyukak keresése** funkciót.
- Egy térkép nem nyílik meg a fájlkezelőből: lehet, hogy egy már futó szerkesztő kapta meg; keresse meg azt az ablakot.

## Beállítások és biztonsági mentések
A beállítások, a legutóbbi térképek, a csillagok, az exportbeállítások és a bővítmények a konfigurációs mappában vannak: Linuxon és macOS-en a `~/.config/heron-brush`, Windowson a `%LOCALAPPDATA%\heron-brush` mappában. Az automatikus biztonsági mentések az egyes térképek melletti `autosave` mappában találhatók.

## Hiba bejelentése
Adja meg a szerkesztő verzióját (Súgó → Névjegy), az operációs rendszerét, egy kis térképet, amely bemutatja a problémát, és a pontos lépéseket. A kimeneti panel (`Ctrl+4`) gyakran megmondja, mi romlott el. Kísérletezés előtt mentsen egy másolatot.

## A kézikönyv használata
Válasszon egy fejezetet a bal oldali tartalomjegyzékben. A `Ctrl+F` megjeleníti a tartalomjegyzéket, és a keresőmezőre lép; az `Esc` törli a keresést, vagy ha a keresés üres, bezárja a kézikönyvet. A **Tartalom** elrejti vagy megjeleníti a fejezetlistát, a **Szövegméret** pedig a betűméretet módosítja; mindkettőt megjegyzi a program. A felül lévő nyelvlista a rendelkezésre álló fordítások között vált.
