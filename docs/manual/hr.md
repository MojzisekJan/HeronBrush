# Prvi koraci
Heron Brush je uređivač razina temeljen na brushevima. Razine gradite od konveksnih brusheva, bojite i teksturirate njihove plohe, postavljate entitete, animirate vrata i platforme te rezultat izvozite u pogon igre. Ima i ugrađene resurse, parametarske oblike, veze uživo s uređivačima pogona, zajedničke sesije uređivanja i renderiranje statičnih slika. Ovaj priručnik isporučuje se s aplikacijom i radi bez interneta.

Na macOS-u `Cmd` radi svugdje gdje ovaj priručnik navodi `Ctrl`, a Option je Alt.

## Početni zaslon
Kad se Heron Brush pokrene bez mape, otvara se početni zaslon. Lijevo su **Nastavi** (posljednja mapa koju ste imali otvorenu), **Nova mapa** i **Pregledaj** za otvaranje postojeće datoteke `.map`, te **Igra**, **Priručnik** i **Postavke**. Desno je popis nedavnih mapa sa sličicama, a ispod njega **Primjeri**. Zvjezdica (**Zadrži na vrhu**) prikvačuje mapu na vrh popisa; kanta (**Premjesti u smeće**) premješta datoteku u smeće sustava, pa se može vratiti. Tipke sa strelicama kreću se po zaslonu, a Enter otvara označenu stavku. Datoteka → Dobro došli u bilo kojem trenutku vraća ovaj zaslon.

## Vaša prva mapa
1. Na početnom zaslonu odaberite **Nova mapa** ili Datoteka → Nova mapa (`Ctrl+N`).
2. Na traci alata odaberite alat Brush (`B`) i povlačenjem u 2D prikazu nacrtajte odabrani oblik. Dubinu provjerite u drugom 2D prikazu.
3. Odaberite brush i povlačenjem ga pomaknite. Na kartici **Ploha** dodijelite mu materijal ili boju.
4. Izvornu mapu spremite s `Ctrl+S`. Kad je budete htjeli koristiti u igri, otvorite Datoteka → Izvoz i zapišite scenu za svoj pogon.

## Datoteke i spremanje
Datoteka `.map` vaš je izvor koji možete uređivati. Spremajte redovito: izvoz, veze uživo i renderiranje nikad ne spremaju mapu umjesto vas. Datoteka → Spremi kao (`Ctrl+Shift+S`) dodaje nastavak `.map` ako ga izostavite. Datoteka → Vrati na spremljenu mapu odbacuje nespremljene promjene i ponovno učitava datoteku. Ako zatvorite prozor ili otvorite drugu mapu dok postoje nespremljene promjene, Heron Brush pita želite li **Spremi**, **Odbaci** ili **Odustani**.

Dok spremljena mapa ima nespremljene promjene, svakih deset minuta zapisuju se numerirane sigurnosne kopije u direktorij `autosave` pokraj mape. Čuva se 50 najnovijih kopija.

Otvaranje mape dok Heron Brush već radi predaje je pokrenutom prozoru, koji dolazi u prvi plan. Kad želite zaseban prozor, u naredbenom retku koristite `--new-instance` (vidi Naredbeni redak).

# Sučelje
Glavni prozor ima naslovnu traku s izbornicima, traku alata ispod nje, prikaze mape, inspektor s desne strane i statusnu traku pri dnu.

## Naslovna traka i izbornici
Naslovna traka prikazuje logotip, izbornike, naziv mape u sredini i gumbe prozora. Na macOS-u su uobičajeni crveni, žuti i zeleni gumbi lijevo. Povlačenjem naslovne trake pomičete prozor, a dvoklikom ga maksimizirate. Dok nešto radi u pozadini, pri kraju trake pojavljuje se pokazatelj: veza uživo s pogonom, sesija uživo ili **MCP aktivan**. Klikom na pokazatelj zaustavljate vezu uživo ili otvarate njezin prozor.

- **Datoteka**: Dobro došli, Nova mapa (`Ctrl+N`), Otvori mapu (`Ctrl+O`), Spremi (`Ctrl+S`), Spremi kao (`Ctrl+Shift+S`), Vrati na spremljenu mapu, Izvoz, Renderiraj sliku, Dodaci i Postavke (`Ctrl+,`).
- **Uredi**: Poništi (`Ctrl+Z`), Ponovi (`Ctrl+Shift+Z`), Izreži, Kopiraj, Zalijepi, Dupliciraj (`Ctrl+D`), Izbriši, Odaberi sve (`Ctrl+A`), Ponovi posljednje naredbe (`Ctrl+R`) i Zaboravi ponovljive naredbe.
- **Prikaz**: 3D prikaz u zasebnom prozoru, Idi na, Prianjanje uz mrežu (`Alt+0`), Prikaži mrežu (`0`), Pregled animacija i Prikaži izostavljene plohe.
- **Geometrija**: Umetni 3D tekst, Spoj u kutu, Pretvori plohe u patcheve, Premjesti odabir iz njegova entiteta i Sinkroniziraj povezane grupe.
- **Animacija**: Animacija ključnim sličicama, Pregled animacija i Postavi kameru prema 3D prikazu (`Ctrl+Alt+0`).
- **Mapa**: Učitaj, Ponovno učitaj i Zatvori point datoteku; Učitaj, Ponovno učitaj i Zatvori datoteku portala. One prikazuju tragove propuštanja i portale koje zapisuju prevoditelji mapa.
- **Suradnja**: Sesija uživo i Posluživanje za agenta (MCP).
- **Pomoć**: Priručnik i O programu.

## Traka alata
Traka alata sadrži sve alate. Prva skupina koristi se stalno: Odaberi, Brush, Rez, Vrh, Brid, Ploha, Rotiraj i Skaliranje. Druga skupina sadrži Smicanje, Sastavi, Provlačenje, Kontrolna točka, Zaobljenje i Bojanje. Zadržite pokazivač iznad gumba da vidite njegov naziv i prečac. Ponovni klik na aktivni alat vraća na Odaberi.

Nakon alata dolaze **Zrcali vodoravno** i **Zrcali okomito**, zatim dva prekidača: **Zaključaj** čuva mapiranje materijala kad se brush pomiče, a **UV** ga čuva kad se brush okreće (`U` ga uključuje i isključuje). Slijede postavke aktivnog alata: alat Brush prikazuje oblik i njegove opcije, a alat Zaobljenje svoj polumjer, plohe i kutove s gumbom **Primijeni**. Na desnom kraju su naziv alata, veličina mreže (**Mreža 16** i slično; kliknite za odabir druge) i izbornik **Postavke mreže** s Prianjanje uz mrežu, Prikaži mrežu, Pregled animacija, Prikaži izostavljene plohe i Animacija ključnim sličicama.

## Prikazi mape
Postoje četiri prikaza: 3D prikaz i tri ravna prikaza, Odozgo (XY), Sprijeda (XZ) i Bočno (YZ). Postavke → Prikaz → Raspored nudi Jedno okno, Dva okna, Tri okna ili Četiri okna. Povlačenjem razdjelnika mijenjate veličinu okana. Klik u prikaz daje mu tipkovnicu. `Space` prebacuje okno koje prikazuje jedan po jedan prikaz na sljedeći prikaz, a `Ctrl+Space` maksimizira prikaz koji ima tipkovnicu i ponovno vraća raspored.

Desni klik bez povlačenja u bilo kojem prikazu otvara kontekstni izbornik za odabir i ono što je ispod pokazivača. Nudi grupiranje, slojeve, povezane grupe, stvaranje entiteta, premještanje brusheva u entitet ili iz njega, kopiranje naziva materijala i njegovo prikazivanje u pregledniku materijala.

## Inspektor
Inspektor s desne strane ima četiri kartice. Povlačenjem njegova lijevog ruba mijenjate mu širinu.

- **Mapa** (`Ctrl+1`): Slojevi, Grupe, Dodaci, Problemi, View filters i Oznake.
- **Entitet** (`Ctrl+2`): preglednik entiteta, definicije entiteta, svojstva odabranog entiteta i, za gumbe i okidače, Okidač animacije.
- **Ploha** (`Ctrl+3`): UV, Materijali, Boja, Provlačenje, Zaobljenje, Patch te Uvlačenje i skošenje. Svaki se odjeljak može sklopiti.
- **Resursi**: biblioteka brush modela za višekratnu upotrebu.

`Ctrl+5` skriva ili prikazuje inspektor, a `Ctrl+Alt+T` skriva ili prikazuje traku alata, čime prikazi dobivaju više prostora.

## Statusna traka i okno izlaza
Statusna traka pri dnu prikazuje najnoviju poruku uređivača. Pogreške su crvene. Kliknite traku ili pritisnite `Ctrl+4` da otvorite okno izlaza s cijelom poviješću poruka. Većina naredbi ovdje javlja što je napravila, primjerice koliko je brusheva odrezano.

## Dijalozi i izgled
Dijalozi poput Postavke, Izvoz i Renderiraj sliku otvaraju se kao zasebni prozori. Postavke → Prikaz → Shema boja nudi Prema sustavu, Svijetla ili Tamna. **Neprozirnost ploča (%)** na stranici Pregled čini ploče prozirnima iznad prikaza. Prozor sam crta svoje gumbe u stilu vašeg sustava: izvorno na macOS-u, u stilu Windows 11 na Windowsima i u stilu GNOME-a na Linuxu.

# Kretanje i tipkovnica
## Kretanje u 3D prikazu
Držite desnu tipku miša i povlačite da se ogledate. Povlačenjem srednjom tipkom pomičete pogled. Kotačić pomiče naprijed i natrag. `W`, `A`, `S` i `D` lete naprijed, lijevo, natrag i desno, `Q` diže, a `X` spušta; držite `Shift` za veće korake. Kad je u Postavke → Miš uključeno **Alt + povlačenje srednjom tipkom pomiče kameru**, Alt sa srednjom tipkom pomiče duž smjera pogleda. Brzine, obrtanja i vidno polje nalaze se u Postavke → Miš.

## Kretanje u 2D prikazima
Povlačenjem srednjom tipkom pomičete pogled, a kotačićem zumirate oko pokazivača. Pomicanje i zumiranje jednog ravnog prikaza drži ostale poravnatima; to određuje **Poveži 2D prikaze** u Postavke → Miš. `Ctrl+Alt+Z` vraća zum, a `Ctrl+Shift+U` vraća ravne prikaze na ishodište.

`Ctrl+U` uokviruje odabir u svim prikazima. Prikaz → Idi na (`Ctrl+Alt+M`) premješta kameru na upisane koordinate, odabire ono što je u određenom retku datoteke mape ili pomiče odabir za upisani iznos.

## Mreža i prianjanje
`+` i `-` mijenjaju veličinu mreže, a brojčane tipke `1` do `9` izravno biraju veličinu od 1 do 256 jedinica. `0` ili `#` prikazuje ili skriva mrežu, a `Alt+0` uključuje ili isključuje prianjanje. `S` prianja vrhove odabranih brusheva uz trenutačnu mrežu, `Ctrl+Shift+V` uz cijele jedinice, a `Ctrl+Alt+Shift+V` uz mrežu.

## Uobičajeni prečaci
- `F3` ili `Ctrl+Shift+P`: traži bilo koju naredbu iz izbornika po nazivu i pokreće je tipkom `Enter`. `F10` otvara izbornike, a strelice se kreću po njima.
- `Ctrl+S`: spremi mapu. `Ctrl+Shift+S`: spremi kao.
- `Ctrl+Z`: poništi. `Ctrl+Shift+Z` ili `Ctrl+Y`: ponovi.
- `Ctrl+C`, `Ctrl+X`, `Ctrl+V`: kopiraj, izreži i zalijepi. Lijepljenje stavlja objekte na mjesto pokazivača; `Ctrl+Alt+V` zadržava njihove izvorne koordinate. Kad su odabrane plohe, kopiranje uzima njihov materijal i UV postavke.
- `Ctrl+D`: dupliciraj. `Ctrl+Shift+D`: povezani duplikat grupe.
- `Delete` ili `Backspace`: izbriši odabir.
- `Esc`: prekida trenutačno povlačenje, vraća korak unatrag u aktivnom alatu, vraća na Odaberi i na kraju poništava odabir. `Shift+Esc` izravno vraća na Odaberi.
- `Ctrl+R`: ponovi posljednje ponovljive naredbe. `Ctrl+Shift+R`: zaboravi ih.
- `Ctrl+Alt+0` ili `Ctrl+Alt+Num0`: postavi kameru za renderiranje na mjesto 3D prikaza.
- `F5`: ponovno učitaj materijale. `F6`: ponovno učitaj definicije entiteta.

## Prečaci za odabir
- `Ctrl+A`: odaberi sve, ili sve ručke dok je aktivan alat za vrhove, bridove ili plohe. `Ctrl+Shift+A`: ne odaberi ništa. `Ctrl+Alt+A`: obrni odabir.
- `Ctrl+B`: odaberi ostale objekte u istom entitetu ili grupi.
- `Ctrl+T`: odaberi sve što odabrani brushevi dodiruju. `Ctrl+E`: odaberi sve unutar njih. `Ctrl+Shift+E`: odaberi sve u njihovu stupcu duž osi pogleda. Ove naredbe troše brusheve kojima se odabire.
- `Ctrl+I`: izoliraj odabir. `Ctrl+Alt+I`: sakrij ga. `Ctrl+Shift+I`: ponovno prikaži sve.
- `Ctrl+G`: grupiraj. `Ctrl+Shift+G`: razgrupiraj.

## Prečaci za transformacije
- Tipke sa strelicama i `Page Up` / `Page Down` pomiču odabir za jedan korak mreže u smjeru u koji pokazuju u aktivnom prikazu. S `Ctrl` pri pomicanju dupliciraju, a s `Alt` rotiraju.
- `Alt+X`, `Alt+Y`, `Alt+Z`: rotiraj za četvrtinu okreta oko te osi; dodajte `Shift` za suprotan smjer.
- `Ctrl+F`: zrcali poprijeko prikaza; `Ctrl+Alt+F`: zrcali gore-dolje. `Ctrl+Shift+X` i `Ctrl+Shift+Y` zrcale duž X ili Y.
- `Insert`: postavi točkasti entitet u središte odabira.
- `Tab`: kad je odabran brush, kruži Odaberi → Vrh → Brid → Ploha → Odaberi.

## Raspored tipki Blender
Postavke → Tipkovnica → Raspored tipki nudi Heron Brush (zadano, koristi se u cijelom ovom priručniku) i Blender. S Blenderom: `G` odabire i pomiče, `R` rotira, `S` skalira, `E` provlači, `1`, `2` i `3` biraju alate za vrhove, bridove i plohe, `K` reže, `B` odabire, `A` odabire sve, a `Alt+A` poništava odabir, `X` briše, `Shift+D` duplicira, `H` skriva, a `Alt+H` prikazuje sve, a `Ctrl+Shift+Z` ponavlja. Sve ostalo je isto u oba rasporeda.

# Brushevi i alati za modeliranje
Brushevi su konveksna tijela određena ravninama. Složene objekte gradite od nekoliko jednostavnih brusheva umjesto da pokušavate napraviti jedan konkavan brush. Gdje je dovoljan jednostavan oblik, držite broj brusheva niskim.

## Crtanje brusheva
Odaberite alat Brush (`B`) i povucite u prikazu. Traka alata prikazuje **Oblik za crtanje**: Kvadar, Stepenice, Svod, Valjak, Stožac, Sferoid (UV), Sferoid (ikosaedar) i Umetni 3D tekst. `Alt+1` do `Alt+7` izravno biraju oblik. Ovisno o obliku traka prikazuje **Šuplje** i **Debljina:**, **Visina stube:** i **Smjer:** za stepenice, **Os:**, **Stranica:** i način kruga za oble oblike, **Prstenova:** za sfere te **Preciznost:** za ikosaedar. Opcije postavite prije povlačenja. Rezultat provjerite u više prikaza.

## Odabir
Kliknite brush da ga odaberete, a `Ctrl`-klikom dodajete u odabir. Povlačenje iz praznog prostora crta okvir odabira. U 3D prikazu `Shift`-klik odabire jednu plohu, a `Ctrl+Shift`-klik dodaje još ploha. Povlačenje odabranog brusha pomiče ga; povlačenje neodabranog odabire i pomiče samo njega.

U 3D prikazu `Shift`-povlačenjem plohe odabranog brusha istiskujete je. `Alt`-klik na drugu plohu kopira na nju materijal i poravnanje odabrane plohe; `Alt+Ctrl` kopira samo materijal, a `Alt+Shift` kopira rotaciju.

## Alati za oblikovanje
- **Odaberi** (`Esc`): odabir i pomicanje.
- **Vrh** (`V`), **Brid** (`E`), **Ploha** (`F`): povlačenje kutova, bridova ili ploha. `Shift`-povlačenje iz sredine brida ili plohe ondje dodaje kut. `Delete` uklanja odabrane kutove.
- **Rotiraj** (`R`), **Skaliranje** (`T`), **Smicanje** (`G`): transformiranje odabira ručkama.
- **Rez** (`C`): kliknite dvije ili tri točke da odredite ravninu reza. `Enter` reže, `Ctrl+Enter` bira koja strana ostaje, a `Backspace` ili `Esc` uklanja posljednju točku.
- **Sastavi** (`N`): postavite točke oko volumena i pritisnite `Enter` da od njih napravite konveksni brush. `Backspace` uklanja posljednju točku.
- **Provlačenje** (`Y`): vidi odjeljak o provlačenju u nastavku.
- **Kontrolna točka** (`P`): preoblikovanje kontrolne mreže patcha.
- **Zaobljenje** (`O`): kliknite brid da ga zaoblite ili odaberite bridove ili vrhove njihovim alatima i pritisnite `Enter`. Više ploha daje glatkiji rezultat i više geometrije.
- **Bojanje** (`K`): vidi Materijali i UV.

## Kombiniranje brusheva
- `Ctrl+J`: spoji odabrane brusheve u njihovu konveksnu ljusku.
- `Ctrl+K`: oduzmi odabrane brusheve od brusheva koje dodiruju.
- `Ctrl+Shift+K`: izdubi odabir, sa stijenkama debljine jednog koraka mreže.
- `Ctrl+L`: presjek odabira.

## Provlačenje, zaobljenje, patch i skošenje
**Provlačenje** gradi geometriju duž putanje od odabranih ploha. Odaberite plohe (u alatu Provlačenje `Alt`-klikom), zatim u odjeljku Provlačenje na kartici Ploha postavite **Putanja kretanja**, **Okreni** (os i kut), **Skaliranje**, **Koraci** i **Putanja** (poravnanje Cijeli broj ili Slobodno; Luk, Ravno ili S-zavoj) i pritisnite **Provlačenje**. Redak ispod kontrola kaže koliko će brusheva nastati.

Odjeljak **Zaobljenje** zaobljuje odabrane bridove s **Polumjer**, **Plohe** i **Kutovi** (Kosi spoj ili Zaobljeno); jedna ploha daje skošeni rub.

**Patch** prikazuje **Stupci** i **Retci** odabranog patcha s gumbima za dodavanje ili uklanjanje para. Geometrija → Pretvori plohe u patcheve (`Ctrl+P`) od odabranih ploha stvara patcheve.

**Uvlačenje i skošenje** radi na odabranim plohama: **Uvlačenje** uvlači plohu od njezinih rubova, a **Dubina** je gura van (ili unutra, kad je negativna). **Skosi plohe** primjenjuje oboje; treći gumb dodaje ploču ili urezuje udubljenje, ovisno o predznaku dubine. **Spoj u kutu** otvara dijalog koji spaja dva brusha u kosom ili zaobljenom kutu, s pregledom.

## 3D tekst
Geometrija → Umetni 3D tekst (ujedno posljednja stavka popisa oblika alata Brush) gradi brusheve od teksta. Koristi Heron Block, ugrađeni poligonalni font s odrezanim kutovima, proporcionalnim širinama i češkim dijakritičkim znakovima; mala slova koriste oblike velikih. Upišite **Tekst**, postavite **Visina** (visina velikih slova; dijakritici se dižu iznad nje) i **Dubina**, pratite pregled uživo i pritisnite **Umetni**.

Ako operacija ne može proizvesti valjanu geometriju, prilagodite njezine dimenzije ili poništite i pojednostavnite oblik.

## Nizovi
Geometrija → Niz kopira odabir i prikazuje kopije na karti dok mijenjate vrijednosti. Brojevi uključuju original, a niz stvara najviše 1024 kopije.

- **Red** pomiče svaku kopiju u odnosu na prethodnu za **Korak X**, **Korak Y** i **Korak Z** i zakreće je za **Zakretanje po kopiji (stupnjevi)** oko odabrane osi.
- **Mreža** popunjava **Broj X**, **Broj Y** i **Broj Z** elemenata s razmacima **Korak X**, **Korak Y** i **Korak Z**.
- **Prsten** raspoređuje elemente unutar **Kut (stupnjevi)** oko središta udaljenog **Polumjer (jedinice)** od odabira. Uz **Okreni prema središtu** svaka se kopija zakreće s prstenom.
- **Raspršenje** postavlja kopije na nasumična mjesta unutar polumjera, poprijeko odabrane osi. Isto **Sjeme** daje isti raspored, a **Nasumično zakretanje** zakreće svaku kopiju.

Kad je odabrana jedna cijela grupa, **Povezane kopije** od svake kopije radi povezani duplikat, pa izmjena jedne stiže do svih. **Primijeni** zadržava kopije i odabire ih zajedno s originalom; **Odustani** ih uklanja. Cijeli niz poništava se jednim korakom.

## Oblikovanje terena
Teren se sastoji od mnogo malih brusheva čiju površinu kist za oblikovanje gura i vuče. Odaberite jedan ili više brusheva pa Geometrija → Podijeli na stupce. Brushevi se režu duž linija mreže na stupce široke jedan korak mreže, a stupci idu u grupu Terrain. Rezovi prate mrežu svijeta, pa se susjedni brushevi režu na istim mjestima i dijele kutove. Svaki stupac stoji duž najtanje strane brusha: okomito u podu ili stropu, vodoravno u zidu. Špilja se radi isto: izdubite kvadar (Hollow selection) s debljinom zida od jednog koraka mreže, odaberite zidove i podijelite ih.

Prozor **Oblikovanje**, koji otvara gumb na kraju trake za oblikovanje, određuje oblik stupaca i može odmah podijeliti odabir. **Trokut** daje dva stupca po ćeliji mreže, a vrh trokuta ostaje ravan kako god ga oblikovali. **Kvadrat** daje jedan stupac po ćeliji, dakle upola manje brusheva; vrh mu se savija duž dijagonale. **Šesterokut** prekriva brush šesterokutima širokima dva koraka mreže, svaki podijeljen na šest trokuta oko središta, pa svaki vrh ostaje ravan.

Odaberite alat **Oblikovanje** (`J`) i povlačite po površini u 3D prikazu. Narančasti prsten prikazuje kist. Bez odabira kist mijenja svaki brush ispod sebe, s odabirom samo odabrane. Traka na vrhu prozora određuje što potez radi:

- **Podigni** i **Spusti** pomiču površinu van ili unutra, najviše u središtu prstena, a na rubu nimalo. Držite `Shift` da ih zamijenite.
- **Izgladi** izravnava neravnine.
- **Poravnaj** poravnava površinu na mjesto gdje je potez počeo.

U istom prozoru **Smjer** određuje kamo:

- **Gore** pomiče površinu ravno gore i dolje, unutar kruga na tlu. To je za teren.
- **Kamera** je pomiče prema pogledu i od njega, unutar kugle oko pokazivača. Pogledajte pod, zid ili strop i oblikuje se ono što gledate.
- **Normala** je pomiče van iz plohe ispod pokazivača na mjestu gdje potez počinje.

Dva broja su **Polumjer (jedinice)** i **Jačina (%)**. Pomiču se samo kutovi na strani koju potez pomiče; stražnja strana ostaje na mjestu, a stupac ostaje debeo barem jednu jedinicu. Kut koji brushevi dijele pomiče se u svima zajedno, pa površina ostaje zatvorena. Potez se poništava jednim korakom. U 2D prikazima alat odabire kao alat Odabir.

Kad završite s oblikovanjem, odaberite teren i izaberite Geometrija → **Spoji stupce** ili gumb u prozoru **Oblikovanje**. Susjedni stupci koji zajedno čine konveksan brush i jednako izgledaju postaju jedan brush, pa ravni dijelovi postaju nekoliko velikih brusheva. Površina se ne pomiče, a spajanje se poništava u jednom koraku.

Za teren kao u voxel igrama podijelite na stupce **Kvadrat** i u prozoru **Oblikovanje** kod **Potez** odaberite **Blokovi**. Potez tada pomiče cijeli vrh svakog stupca za cijele korake mreže i ostavlja stranice okomitima, pa teren raste u stepenicama. Svi načini rade kao prije; mali potezi se zbrajaju dok stupac ne naraste za cijeli korak. Stupci drugih oblika ostaju kakvi jesu.

**Slobodno** ne treba stupce i radi na bilo kojim brushevima. Podigni dodaje teren u kugli oko pokazivača, a Spusti ga oduzima, pa teren može imati prevjese i špilje. Izgladi ga ujednačuje, a Poravnaj ga dovodi do razine na kojoj je potez počeo. Svaki dodir iznova gradi zahvaćene brusheve na mreži, zaobljene, a sve izvan kugle ostavlja kakvo je bilo, uključujući oštre rubove. Rezultat su obični brushevi, pa na njima rade svi drugi alati, a **Spoji stupce** poslije smanjuje njihov broj. Gušća mreža daje glađu površinu i više brusheva.

# Materijali i UV
## Učitavanje i primjena materijala
Otvorite odjeljak **Materijali** na kartici Ploha. **Učitaj direktorij** dodaje direktorij sa slikama, a **Učitaj .wad** dodaje paket tekstura; oboje se pojavljuje pod **Zbirke**, gdje ih križić uklanja. Upisivanjem u **Filtriraj materijale** sužavate popis. Kliknite materijal da postane trenutačni, a zatim ga **Primijeni na odabir** stavlja na svaku odabranu plohu. **Odaberi** nudi Odaberi brusheve s njim i Odaberi plohe s njim. **Zamijeni** zamjenjuje jedan materijal drugim u cijeloj mapi ili samo u odabiru.

Alat **Bojanje** (`K`) stavlja trenutačni materijal na plohu ispod pokazivača: klik boji jednu plohu, dvoklik boji cijeli brush, a `Shift`-klik uzima materijal plohe kao trenutačni, poput kapaljke.

## UV uređivač
Odjeljak **UV** na kartici Ploha prikazuje materijal i obris odabrane plohe. Upravljanje mišem na platnu:
- Kotačić zumira oko pokazivača. Povlačenjem desnom ili srednjom tipkom pomičete pogled. Dvoklik desnom tipkom ponovno uokviruje plohu; odabir druge plohe uokviruje je automatski.
- Povlačenje lijevom tipkom pomiče materijal dok obris plohe ostaje na mjestu. Prsten rotira mapiranje, kutne ručke ga skaliraju, a središnja ručka postavlja ishodište tih promjena.
- `Alt`-povlačenje lijevom tipkom po rubu pločice smiče, kao i `Shift`-povlačenje srednjom tipkom.
- Svako povlačenje jedan je korak poništavanja, a `Esc` prekida povlačenje u tijeku.

Ispod platna su **Pomak**, **Skaliranje**, **Kut** te, kad ih igra koristi, **Zastavice sadržaja**, **Zastavice površine** i **Vrijednost površine**. Gumbi su **Resetiraj**, **Poravnaj prema svijetu**, **Prilagodi**, **Zrcali vodoravno**, **Zrcali okomito** i dva okreta za 90°. **Poravnaj** okreće materijal duž jednog od bridova plohe (ponovno pritisnite za sljedeći brid, `Shift` za prethodni). Četiri gumba za poravnanje guraju materijal uz rub. **Prilagodi U** i **Prilagodi V** skaliraju tako da plohu pokriva cijeli broj kopija; ponovno pritisnite za sljedeći broj i držite `Ctrl` da zadržite podjelu trim sheeta.

## UV tipke
Kad su plohe odabrane, tipke sa strelicama pomiču materijal, `[` i `]` okreću ga za 15°, `,` i `.` prepolovljuju ili udvostručuju njegovo mjerilo, `Ctrl+F` i `Ctrl+Alt+F` ga zrcale, `Shift+R` ga resetira, `Alt+Shift+R` ga vraća na osi svijeta, a `Ctrl+Shift+F` ga prilagođava svakoj plohi.

**Zaključaj** na traci alata čuva mapiranje dok se brushevi pomiču, a **UV** (`U`) ga čuva dok se brushevi okreću.

## Alatni materijali
- skip: uklanja označenu plohu iz izvoza za pogone.
- clip: stvara sudare bez vidljive geometrije.
- origin: označava središte okretanja brush entiteta.

Nazivi se mogu promijeniti na kartici **Alatni materijali** u prozoru Izvoz. Skip koristite za plohe za koje znate da su skrivene.

## Izostavljanje ploha iz izvoza
Odaberite plohu i u odjeljku UV uključite **Izostavi plohu iz izvoza**. Narančasto šrafiranje označava je u UV pregledu. Njezin materijal i boja ostaju spremljeni, a brush ostaje zatvoren i može se uređivati. Izvezene mreže za renderiranje izostavljaju tu plohu, a sudari se zadržavaju. Izostavljanje jednog četverokuta kocke daje dva trokuta za renderiranje. Ovo ne čini plohe dvostranima niti spaja mreže.

**Izostavi pokrivene plohe** označava plohe koje statični brushevi potpuno pokrivaju, uključujući istovjetne plohe koje se dodiruju i zatvorene brusheve. Radi na odabranim brushevima ili, kad ništa nije odabrano, na svoj vidljivoj otključanoj geometriji svijeta. Brush entiteti, prozirni materijali i brushevi koji već imaju izostavljene plohe preskaču se. Djelomično pokrivene plohe ne dijele se. Jedno poništavanje vraća sve. Prikaz → Prikaži izostavljene plohe skriva ili prikazuje takve plohe u prikazima uređivača; odabir i dalje koristi cijeli brush.

# Boja i prijelazi
Odjeljak **Boja** na kartici Ploha boji plohe jednobojno ili prijelazima preko njihovih materijala. Koristan je za grubo oblikovanje razine i za stiliziranu grafiku. Obojene plohe zadržavaju svoje stvarne boje u prikazima; odabir se prikazuje obrisom.

## Paleta i boja miješanja
Kliknite jedan od dvanaest uzoraka palete da njime obojite odabrane plohe ili cijele odabrane brusheve. **Neprozirnost** postavlja koliko je sljedeće bojanje neprozirno, u postocima. **Miješaj** ima gumb boje koji otvara birač boja; povlačite u biraču i odabir prati uživo, ili pritisnite **Bojanje** da boju miješanja primijenite kao jedan korak. **Natrag na materijal** uklanja boju pa ploha ponovno prikazuje svoj materijal. Redak na vrhu odjeljka kaže što je odabrano i što nosi.

## Birač boja
Svaki gumb boje u uređivaču otvara isti birač. Povlačenjem u kvadratu birate zasićenost i svjetlinu, a u traci pokraj njega nijansu. Ispod su vrijednosti **R**, **G** i **B** te klizač **Neprozirnost** gdje ga boja ima. Heksadecimalno polje prima `#RRGGBB` ili `#RRGGBBAA`; postaje crveno dok tekst nije valjana boja. **Kopiraj** stavlja heksadecimalni kod u međuspremnik. Kliknite izvan birača da ga zatvorite.

## Prijelazi
**Prijelaz** bira Linearni prijelaz ili Radijalni prijelaz, a gumb pokraj njega zamjenjuje krajnje boje. Svaka točka ima gumb boje i **Položaj (%)**, a točke između krajeva imaju gumb za uklanjanje. **Dodaj točku boje** dodaje točku u najveći razmak; prijelaz može imati do osam točaka. **Kut prijelaza** postavlja smjer linearnog prijelaza, s gumbima **+45°** i **−45°**.

Ispod pregleda je traka s oznakom za svaku točku. Kliknite traku da dodate točku, kliknite oznaku da otvorite njezin birač, povucite oznaku da je pomaknete i desnom tipkom kliknite oznaku da je uklonite. Tri uzorka ispod trake su predlošci: Topli prijelaz, Hladni prijelaz i Sivi prijelaz. Promjene prijelaza primjenjuju se na odabir uživo.

Smjer prijelaza je važan: usmjerite ga duž nagiba krova ili duž zakrivljenog objekta, a ne preko nepovezanih ploha. Nakon bojanja provjerite model s obje strane.

## Materijal površine
**Materijal površine** postavlja **Hrapavost** (0 je sjajno, 100 je mat), **Metalnost** (0 za drvo, kamen ili plastiku, 100 za metal) i **Emisija** (0 do 8; površina svijetli vlastitom bojom, uključujući prijelaze, ali ne osvjetljava obližnje objekte). Promjene se uživo primjenjuju na odabrane plohe ili brusheve. **Koristi zadanu površinu** uklanja nadjačavanje. Sve se promjene mogu poništiti.

Površinski efekti vide se u osvijetljenom 3D prikazu i u renderima kamere. Ravni i žičani prikaz namjerno izostavljaju odsjaje. Bloom može dodati sjaj oko emisivnih površina. Godot i glTF/GLB dobivaju vrijednosti materijala, a uvoznik za Unity pretvara hrapavost u glatkoću. Ograničenja svakog cilja opisuje `docs/surface-materials.md`.

# Resursi, slojevi i grupe
## Biblioteka resursa
Otvorite karticu **Resursi**. Direktorij s modelima `.map` dodajete s **Dodaj direktorij**; ugrađena biblioteka uvijek je tu. Za pretraživanje upisujte u **Filtriraj resurse**. Kliknite model da ga odaberete, zatim ga dvokliknite, pritisnite `Enter` ili kliknite **Umetni** da ga dodate ondje gdje je pokazivač posljednji put bio. Model možete i povući u prikaz. Brushevi resursa umeću se kao jedna grupa pa se model pomiče u cjelini.

**Spremi odabir kao resurs** zapisuje odabrane brusheve u biblioteku kao `.map`. Resursi iz biblioteke obični su brushevi koji se mogu uređivati, a ne uvezene mreže.

## Slojevi
Popis **Slojevi** na kartici Mapa organizira velika područja. Upišite naziv i kliknite **Dodaj** da napravite sloj ili pritisnite `Enter` da preimenujete trenutačni. Svaki se sloj može sakriti, zaključati ili izostaviti iz izvoza. **Premjesti ovamo** premješta odabir u trenutačni sloj. Kontekstni izbornik u prikazima nudi i Premjesti u sloj, Postavi kao aktivni sloj, Sakrij slojeve i Izoliraj slojeve.

## Grupe i povezane grupe
`Ctrl+G` grupira odabir, a `Ctrl+Shift+G` ga razgrupira. Odjeljak **Grupe** na kartici Mapa navodi grupe i nudi **Otvori**, **Preimenuj**, **Spoji**, **Razgrupiraj**, **Izvadi** i **Izdvoji**. Povezana grupa skup je kopija koje dijele izmjene: `Ctrl+Shift+D` stvara povezani duplikat, Geometrija → Sinkroniziraj povezane grupe ažurira kopije, a **Prekini vezu** čini kopiju neovisnom. Na kartici Entitet svojstvo se može zadržati kao vlastito svojstvo jedne kopije pri sinkronizaciji povezanih grupa.

## Zajedničko mjerilo
Zadano mjerilo izvoza je 32 jedinice mape po metru. Ugrađena oznaka početka igrača visoka je 64 jedinice, što je referenca od 2 metra. S njom usporedite vrata, vozila, namještaj i biljke. Mjerilo izvoza mijenjajte samo namjerno i držite povezane resurse dosljednima.

# Entiteti i igrivost
Uobičajene definicije entiteta ugrađene su i uvijek dostupne: svjetla, počeci igrača, vrata, gumbi, platforme, okidači, statični rekviziti, zvukovi i točke puta. Učitavanje FGD-a projekta pomoću **Definicije entiteta** na kartici Entitet zamjenjuje klase istog naziva i zadržava ostale ugrađene.

## Postavljanje entiteta
U pregledniku **Entiteti** na kartici Entitet upisujte u **Filtriraj klase** i odaberite klasu. Za točkastu klasu, poput svjetla, kliknite **Postavi** ili dvokliknite klasu; `Insert` postavlja jedan entitet u središte odabira. Za brush klasu, poput vrata ili okidača, odaberite brusheve i kliknite **Stvori**. Klase se mogu i povući u prikaz.

## Svojstva
Popis svojstava uređuje odabrani entitet. Entiteti iste klase uređuju se zajedno. **Dodaj svojstvo** dodaje ključ koji definicija možda ne poznaje; **Zadane vrijednosti** može vratiti postojeća svojstva na zadane vrijednosti ili dodati ona koja nedostaju. Svojstva koja klasa definira, a entitet ih nema, prikazana su zatamnjeno; upisivanjem vrijednosti dodajete ih. Za povezivanje entiteta koristite targetname i target.

## Što izvoz može
Direktive za Godot stvaraju izvorne čvorove, tijela za sudare i područja okidača. Unity i Unreal podržavaju izvorna uobičajena svjetla, a Unreal stvara i aktore PlayerStart. Ostali podaci čuvaju se koliko to uvoznik cilja dopušta.

Sam entitet vrata ne dodaje ponašanje u igri. Kretanje, reakcije na okidače, upravljanje igračem i skripte projekta i dalje zahtijevaju rad u pogonu. Defold izvozi svojstva entiteta i objekte za sudare; Flax postavlja modele i generičke entitete. Prije nego što na rezultatu izgradite igrivost, provjerite ga u svom pogonu.

# Animacija
Heron Brush može pregledati jednostavno kretanje i uređivati animacije brush entiteta ključnim sličicama, a zatim ih izvesti u nekoliko pogona.

## Pregled animacija
Uključite **Pregled animacija** u izborniku Prikaz, izborniku Animacija, izborniku Postavke mreže ili u odjeljku Prikaz na kartici Mapa. Reproducira rotaciju func_rotate i kretanje naprijed-natrag func_shuttle kad je start_on uključen, koristeći svojstva entiteta za os, brzinu, smjer, put i čekanje, te reproducira klipove ključnih sličica postavljene na Reproduciraj pri pokretanju. Resursi ventilatora i vjetrenjače već sadrže dijelove koji se okreću.

Isključivanje pregleda vraća izvornu pozu. Klik u prikazu zaustavlja reprodukciju prije odabira ili uređivanja; kretanje kamerom i dalje radi. Pregled ne mijenja mapu, povijest poništavanja ni izvoze te ne pokreće fiziku ni logiku igre.

## Uređivač ključnih sličica
Otvorite ga s Animacija → Animacija ključnim sličicama ili iz izbornika Postavke mreže. Pristaje ispod prikaza ili u 3D prozor dok je on zaseban. Ikona prozora u njegovu zaglavlju premješta ga u zaseban prozor; zadržava sličicu i povijest poništavanja. **Uredi animaciju** prebacuje prikaze na animiranu pozu, a **Uredi mapu** vraća se na geometriju u mirovanju. U načinu animacije svaki prikaz pri dnu pokazuje **Uredi animaciju** i broj sličice.

Odaberite brush entitet ili brush ili grupu brusheva da ih animirate. Ravna grupa brusheva dijeli jednu animaciju i središte okretanja; grupe koje sadrže više entiteta trebaju zasebne animacije.

## Klipovi
Birač **Animacija** prebacuje između imenovanih klipova. Upišite naziv u **Naziv animacije** i kliknite **Spremi kao** da kopirate trenutačni klip pod tim nazivom; **Izbriši** uklanja klip. **Pomak** stvara klip `translate` bez rotacije. **Animacije vrata** stvara glatke klipove `open` i `close` s okretom od 90 stupnjeva oko trenutačnog središta okretanja; najprije premjestite središte na šarku. **Reproduciraj pri pokretanju** pokreće klip kad razina započne; predložak vrata to isključuje.

## Upravljanje reprodukcijom
Gumbi za upravljanje su **Početak**, **Prethodni ključ**, **Reprodukcija**, **Sljedeći ključ** i **Kraj**. **Sličica** prikazuje i postavlja trenutačnu sličicu, a **Trajanje (s)** postavlja duljinu klipa. Način je **Jednom**, **Petlja** ili **Naprijed-natrag**.

## Vremenska crta
Vremenska crta ima ravnalo s glavom reprodukcije i traku za **Položaj** (X, Y i Z s ključevima zajedno) te za svaki kut rotacije, RX, RY i RZ. Klikom ili povlačenjem po ravnalu pomičete glavu reprodukcije i pregledavate tu pozu u svim prikazima. Povlačenjem srednjom tipkom pomičete vremensku crtu. Povlačenjem ključeva pomičete ih u vremenu; `Shift` dodaje u odabir; dvoklik na traku umeće ključ; `Ctrl`+kotačić zumira. **Krivulje** umjesto traka prikazuje krivulje vrijednosti.

Gumbi iznad nje su **Dodaj ključ**, **Ukloni**, **Kopiraj**, **Zalijepi** i **Dupliciraj**. **Prianjanje uz sličice** drži ključeve na cijelim sličicama, a **Prilagodi** prikazuje cijeli klip. **Promijeni vremena ključeva** otvara **Vremensko mjerilo**, **Pomak (sličice)**, **Skaliraj od glave reprodukcije**, **Produlji trajanje** i **Promijeni vremena odabira**.

Inspektor ključa pokraj vremenske crte uređuje odabrani ključ: **Sličica**, **Vrijednost** (ili tri koordinate ključa položaja), interpolacija **Linearno**, **Zadržavanje** ili **Bézierova**, **Ulazni nagib** i **Izlazni nagib** te **Zagladi tangente**.

## Tipke u uređivaču
Dok vremenska crta ima tipkovnicu: `Space` reproducira ili pauzira, lijeva i desna strelica pomiču za jednu sličicu, `Ctrl+A` odabire sve ključeve, `Ctrl+C`, `Ctrl+X` i `Ctrl+V` kopiraju, izrezuju i lijepe ključeve, `Ctrl+D` ih duplicira, `Delete` ih uklanja, `Ctrl+Z` poništava, a `Esc` prekida povlačenje. Izvan vremenske crte `Esc` izlazi iz načina animacije. Poništavanje ostavlja zum, odabir i glavu reprodukcije gdje su bili.

## Poziranje u prikazima
Redak ispod vremenske crte ima **Onion skinning** s brojem i **Razmak**, što prikazuje susjedne poze kao duhove; koordinate **Pivot**; **Automatski ključevi**; i alate za poziranje **Pomak**, **Rotiraj** i **Pivot**.

- **Dodaj ključ** uključuje Automatski ključevi i pauzira reprodukciju. Kad su Automatski ključevi uključeni i odabran je **Pomak**, povucite obojenu os ručke odabranog entiteta u 3D prikazu da postavite ključ njegova položaja na trenutačnoj sličici.
- U 2D prikazu povucite odabrani brush ili člana njegove grupe da postavite ključ pomaka duž dviju vidljivih osi, uz prianjanje uz mrežu. Za to Automatski ključevi ne moraju biti uključeni.
- S **Rotiraj** povucite brush u 2D prikazu oko njegove narančaste oznake središta okretanja. Odozgo postavlja ključ RZ, Sprijeda RY, a Bočno RX, uz prianjanje uz kutove mreže.
- S **Pivot** povucite žuto središte rotacije u bilo kojem 2D prikazu. Prianja uz mrežu i zadržava svoju dubinu. Ključevi i geometrija u mirovanju ostaju nepromijenjeni.

Cijan putanja u 2D i 3D prikazuje kretanje središta okretanja. Kliknite točku putanje da odaberete njezino vrijeme i ključeve ili je povucite da promijenite položaj; s uključenim automatskim ključevima točke putanje možete povlačiti i u 3D prikazu. Oznake prikazuju brojeve sličica, a isprekidani segmenti označavaju skokove. `Esc` prekida svako od tih povlačenja, a jedno poništavanje vraća cijelo povlačenje.

Nove animacije počinju u mirovanju s ključevima u vremenu nula. Kretanje stvarate promjenom poze na drugoj sličici. Za oštru putanju u obliku slova L postavite ključeve na početak, kut i kraj te koristite Linearno s Naprijed-natrag.

## Povezivanje gumba s animacijama
Odaberite func_button, trigger_multiple ili trigger_once. Kartica Entitet iznad svojstava prikazuje **Okidač animacije**. Odaberite cilj s popisa ili kliknite **Odaberi cilj u prikazu** pa animirani objekt u bilo kojem prikazu; `Esc` prekida odabir. Ciljevi ili klipovi koji nedostaju javljaju se ispod kontrola. Veze entiteta prikazuju strelice prema svojim ciljevima kad je prikaz veza uključen.

**Radnja** bira što aktivacija čini: **Reproduciraj** pokreće imenovani klip ili odabranu animaciju; **Uključi/isključi** izmjenjuje `open` i `close` kad postoje, a inače ponovno pokreće odabranu animaciju; **Otvori** i **Zatvori** pokreću te klipove; **Pauziraj** zadržava pozu, **Nastavi** nastavlja, a **Zaustavi** vraća izvornu pozu. **Samo jednom** nadjačava zadanu vrijednost klase. **Odgoda ponavljanja (s)** postavlja najkraće vrijeme između aktivacija; ne odgađa prvu.

**Isprobaj okidač** izvodi radnju u pregledu bez uređivanja mape ili dodavanja koraka poništavanja. **Vrati pregled** vraća izvorne poze i briše stanje jednokratnosti i odgode. Ovo ne simulira sudare igrača ni skripte igre.

## Animacija u izvozima
Godot izvozi izvorne resurse AnimationPlayer i AnimationLibrary, uključujući krivulje i pokretne sudare. CharacterBody3D koji uđe u područje gumba aktivira njegov cilj. Izmjene tih klipova u Godotu ne vraćaju se u mapu i zamjenjuje ih sljedeći izvoz.

Izvoz za raylib dodaje `<map>_animation.h` s reproduktorom u C-u i `<map>_preview.c` s pregledom slobodnom kamerom; za izgradnju slijedite `<map>_raylib.md`. U pregledu `SPACE` pauzira, `R` ponovno pokreće zadane klipove, a `N` kruži kroz imenovane klipove. Defold ugrađuje reprodukciju u skriptu entiteta s dokumentiranim porukama `heron_*_animation`. Build Level u Unityju dodaje `HeronBrushAnimation` i kinematičko tijelo. Izvoz za Bevy uključuje `heron_animation.rs` i upute. Godot, Unity, Defold, Bevy i raylib čuvaju veze okidača, oznaku jednokratnosti i odgodu ponavljanja. Vaša igra i dalje mora registrirati igrača i povezati svoje gumbe s API-jem svakog okruženja.

## Primjer Motion lab
Otvorite `motion_lab` iz Primjeri na početnom zaslonu. Sadrži platformu s putanjom u obliku slova L, dizalo, rotirajuću platformu i vrata na šarkama s gumbima. Otvorite `godot/project.godot` pokraj njega u Godotu i pritisnite `F5`. WASD i miš pomiču igrača, Space skače, 1 do 4 vode na postaje, a R ponovno pokreće trenutačnu postaju. Stanite na obojenu podlogu da aktivirate njezin cilj. Priloženi README objašnjava ponovni izvoz.

# Izvoz u pogon
Datoteka → Izvoz otvara prozor Izvoz. Pogoni su navedeni lijevo s logotipima: Godot, Defold, three.js, Bevy, raylib, Unity, Unreal Engine, Flax Engine, Cocos Creator, Wavefront .obj, glTF .gltf, GLB .glb i na kraju `.map`. Stranica desno kaže što će cilj dobiti. **Zapiši** ga zapisuje, **Provjeri** gradi razinu bez zapisivanja i navodi probleme, a **Zatvori** zatvara prozor.

Izvoz je odvojen od spremanja: `.map` ostaje vaš izvor koji možete uređivati. Svaka kontrola sprema postavke svog pogona čim se promijeni, pa svaki pogon čuva vlastite odgovore, a prozor se otvara onako kako ste ga ostavili.

## Uobičajene opcije
- **Skaliranje**: **Jedinica po metru**, zadano 32.
- **Što zapisati**: **Geometrija sudara**.
- **Optimizacije**: **Grupiraj statične mreže u ćelije od 1024 jedinice**, **Odbaci plohe zakopane unutar geometrije**, **Spoji plohe u istoj ravnini** i **Spoji oblike sudara**.
- **Putanje projekta**: kamo unutar projekta idu razina i njezini resursi.
- **Alatni materijali**: nazivi materijala clip, skip i origin.
- **Više**: prekidači specifični za pogon, poput **Imenuj čvorove prema**, **Ponovno izgradi slojeve i grupe kao nadređene čvorove**, **Obrni redoslijed vrhova trokuta**, zrcaljenja koordinata tekstura i drugog UV skupa za zapečeno svjetlo s **Teksel svjetlosne mape**.
- **Praćenje mape**: **Ponovno zapiši pri svakom spremanju mape**. Postaje dostupno nakon prvog izvoza za pogon jer taj izvoz određuje kamo se zapisuje.

Statične ćelije dodjeljuju brusheve svijeta prema njihovim središtima i unutar svake ćelije od 1024 jedinice spajaju ih prema materijalu, pa pogoni mogu odbacivati manje dijelove mape. Sudari se zadržavaju, a entiteti igrivosti ostaju zasebni. To može značiti više poziva za crtanje nego jedna mreža svijeta, pa je zadano isključeno. Samostalni izvozi OBJ, glTF i GLB ne koriste ćelije.

## Napomene o ciljevima
- Godot: putanje `res://` tekstura i materijala, fizički sloj okidača i UV2 za svjetlosne mape. Odaberite direktorij izvoza unutar Godot projekta.
- Defold: direktorij resursa, materijal modela i uzorkivač tekstura, koji mora postojati u odabranom materijalu.
- Unity: OBJ mreže i uvoznik za uređivač; izvozite unutar direktorija Assets projekta.
- Unreal Engine: generirana skripta za uvoz i putanja sadržaja, obično /Game/<level>.
- Flax Engine: putanja sadržaja modela relativna prema direktoriju Content projekta.
- three.js, Bevy, raylib i Cocos Creator: scene ili kôd za ta okruženja, s uputama pokraj izvoza.
- glTF / GLB: prenosiva geometrija za kompatibilne alate; GLB je binarni oblik. OBJ: geometrija i materijali za alate za modeliranje.
- `.map`: **Zapiši kao** bira Valve 220, Standardno, Quake 2 ili Quake 2 (Valve). Prozor kaže što bi odabrani dijalekt izgubio.

Nakon zapisivanja pročitajte upozorenja, primjerice o teksturama koje nedostaju. **Što je provjera pronašla** navodi ih u prozoru.

# Veze uživo: Godot, Unity, Defold i Flax
Veza uživo šalje nespremljene izmjene iz Heron Brusha u uređivač pogona dok radite. Jednosmjerna je: izmjene napravljene u pogonu ne vraćaju se. Po prozoru mape može raditi jedna veza uživo. Zaustavite je prije promjene pogona i ponovno je pokrenite da primijenite promijenjene opcije izvoza. Kontrole su u prozoru Izvoz, na stranici pogona, na kartici **Veza uživo**.

Pokazatelj u naslovnoj traci prikazuje stanje veze; kliknite ga da zaustavite vezu. Zatvaranje ili zamjena mape zaustavlja objavljivanje. Nakon neuspjelog uvoza riješite problem i ponovno pokrenite vezu.

## Godot
1. Jednom izvezite u Godot projekt.
2. Kliknite **Instaliraj proširenje za Godot**.
3. Uključite Heron Brush Live u Godotovim Project Settings → Plugins.
4. Kliknite **Pokreni vezu uživo** i iz Godotova panela priključite mapu na otvorenu 3D scenu.

Proširenje zamjenjuje samo svoju generiranu granu i zadržava položaj priključka i ostale čvorove. Godot scenu spremite sami, a vlastite skripte i čvorove držite izvan generirane grane.

## Unity
1. Jednom izvezite unutar direktorija Assets Unity projekta.
2. Kliknite **Instaliraj proširenje za Unity** i pustite da ga Unity prevede.
3. U Unityju otvorite Window → Heron Brush → Live Link.
4. Pokrenite vezu i priključite mapu na aktivnu Unity scenu.

Prozor veze uživo u Unityju držite otvorenim. Ažuriranja se pauziraju u Play Mode i Prefab Mode. Zamjenjuje se samo podređeni objekt Generated objekta mape. Ne uređujte generirane datoteke.

## Defold
1. Jednom izvezite u svoj Defold projekt i držite projekt otvorenim u Defoldu.
2. Kliknite **Pokreni vezu uživo**. Proširenje nije potrebno.
3. U Defoldu otvorite heron_brush_live/preview.collection ili ga referencirajte iz vlastite scene.

Veza zapisuje u zaseban generirani direktorij i ne dira konačni izvoz ni game.project. **Ažuriraj pokrenutu igru u hodu** ažurira i igru pokrenutu iz Defolda; dodavanje objekata ili promjena sudara mogu zahtijevati ponovno pokretanje razine. Neuspjela povezivanja automatski se ponavljaju.

## Flax
Izvezite u direktorij Content projekta i kliknite **Instaliraj proširenje za Flax**. Pustite da ga Flax prevede, pokrenite vezu, a zatim u otvorenoj Flax sceni odaberite Tools → Heron Brush: attach live map. Generirani korijen zadržava svoj položaj, a ostali objekti ostaju netaknuti. Scenu spremite sami. Ažuriranja se pauziraju u Play Mode. Tools → Heron Brush: stop live map pauzira primanje.

# Zajedničke sesije uživo
Suradnja → Sesija uživo otvara prozor za ugošćivanje drugog korisnika Heron Brusha ili pridruživanje njemu. Time više osoba zajedno uređuje mapu; to je odvojeno od veze uživo s pogonom.

## Ugošćivanje i pridruživanje
Upišite **Vaše ime**. Za ugošćivanje odaberite **Ugosti sesiju**, kliknite **Pokreni sesiju** i gostima pošaljite podatke iz **Kopiraj podatke za povezivanje**. Za pridruživanje odaberite **Pridruži se sesiji**, zalijepite podatke i kliknite **Pridruži se**. Pridruživanje zamjenjuje otvorenu mapu domaćinovom, pa najprije spremite svoj rad.

Standardna verzija radi na lokalnoj mreži, a **Port lokalne mreže** postavlja port. Sesije putem interneta (**Poveži se putem interneta**) zahtijevaju verziju s neobaveznom mrežnom značajkom. Sudionici u 3D prikazu vide oznake kamera i imena drugih.

Zatvaranje prozora ne prekida sesiju; pokazatelj u naslovnoj traci prikazuje njezino stanje. **Zaustavi sesiju** ili **Napusti sesiju** je završava. Kad dvije osobe uređuju isti objekt, redoslijed domaćina određuje konačnu verziju, a i poništavanje je zajednička izmjena. Gdje možete, radite na odvojenim područjima i spremajte kontrolne točke.

# 3D prikaz i drugi monitor
## Načini prikaza
Odjeljak **Prikaz** na kartici Mapa određuje što prikazi crtaju: Brushevi, Točkasti entiteti, Modeli entiteta, Nazivi klasa entiteta, Obrisi grupa, Patchevi, Materijali, Prikaži izostavljene plohe, Pregled animacija, Žičani model, Bridovi i Obrisi brush entiteta. Žičani model crta bridove trokuta, uključujući dijagonale i bridove iza površina, bez materijala. To je privremeni prikaz za otklanjanje pogrešaka i ne utječe na izvoze ni rendere.

Osvjetljenje, sjene, ambijentalna okluzija, odrazi, bloom i zaglađivanje za 3D prikaz nalaze se u Postavke → Renderer. To su postavke pregleda; konačne slike koriste vlastite postavke kamere za renderiranje.

## Zaseban 3D prozor
Prikaz → 3D prikaz u zasebnom prozoru premješta 3D prikaz u zaseban prozor koji možete staviti na drugi monitor. Dijeli kameru, dokument i alate, a ravni prikazi zauzimaju prostor u glavnom prozoru. Njegov izbornik nudi Materijali, Puni prikaz i Žičani model; podizbornik Prikaz s Prikaži izostavljene plohe, Pregled animacija, Bridovi, Obrisi grupa, Točkasti entiteti, Modeli entiteta i Obrisi brush entiteta; Fokusiraj na odabir; Sprijeda, Bočno i Odozgo, koji okreću kameru oko odabira (ili točke ispred kamere); Postavi kameru prema 3D prikazu; Cijeli zaslon (`F11`); Renderiraj sliku; i Vrati u glavni prozor. Zatvaranje prozora ponovno vraća prikaz na mjesto.

## Proceduralno nebo
Proceduralno nebo zadano je uključeno u 3D prikazu i može se isključiti u Postavke → Renderer → Svjetlo. Kamere za renderiranje imaju vlastiti prekidač **Proceduralno nebo**, koji se sprema s mapom. Boje neba, obzora i tla dolaze iz postavki okruženja, a sunce prati osvjetljenje scene. Nebo nema geometriju i ne izvozi se.

# Kamere za renderiranje i PNG slike
Datoteka → Renderiraj sliku otvara prozor za renderiranje: kameru i njezine postavke lijevo i pregled uživo desno, koji se ponovno crta dok ih mijenjate. Kamere se spremaju u `.map` u sloju Render cameras koji postoji samo u uređivaču i izostavlja se iz izvoza za pogone.

## Postavljanje kamere
Uokvirite kadar u 3D prikazu i pritisnite `Ctrl+Alt+0` (ili `Ctrl+Alt+Num0`) ili kliknite **Postavi kameru prema 3D prikazu**. Ako nijedna kamera nije odabrana, ažurira se prva spremljena; ako nijedna ne postoji, stvara se nova. Ovo se može poništiti i zadržava postavke izlaza i osvjetljenja. **Odaberi kameru u mapi**, **Učitaj kameru iz mape** i **Premjesti 3D prikaz na kameru** rade obrnuto. **Okvir kamere u 3D prikazu** prikazuje granicu izlaza, a **Upravljaj kamerom iz 3D prikaza** omogućuje da uobičajeno kretanje pomiče spremljenu kameru.

## Postavke
- **Položaj kamere**: X, Y, Z, zakret i nagib.
- **Izlaz i objektiv**: **Predlošci** (Full HD, Kvadrat, Okomito), širina, visina, uzorci, **Okomito vidno polje** i **Daleka ravnina odsijecanja**.
- **Osvjetljenje**: Dnevno svjetlo, Zalazak sunca ili Oblačno, ili postavite ekspoziciju, sunce, boje neba, obzora i tla, **Proceduralno nebo**, **Sjene**, **Ambijentalna okluzija** i **Bloom**.
- **Dubinska oštrina**: **Uključi dubinsku oštrinu**, udaljenost fokusa, raspon oštrine i polumjer zamućenja. **Fokusiraj na odabir** fokusira na središte odabranog objekta ispred kamere.

**Spremi kameru i postavke** sprema obrazac u mapu; spremite mapu da ga zadržite na disku.

## Renderiranje
**Renderiraj PNG** pita gdje spremiti i pokreće progresivno renderiranje, koje se u pregledu popunjava uzorak po uzorak. Više uzoraka izgleda bolje, ali traje dulje. **Prekini renderiranje** zaustavlja bez zapisivanja datoteke. Renderer koristi rasterski cjevovod uređivača, a ne praćenje putanja. PNG ima neprozirnu pozadinu, a animacije se ne renderiraju. Vrlo velike slike mogu nakratko zaustaviti uređivač.

# Postavke
Datoteka → Postavke (`Ctrl+,`) otvara postavke. Promjene se primjenjuju odmah i spremaju se. Bočna traka navodi Pregled, Igre, Prikaz, Boje, Miš, Tipkovnica i Renderer. Polje za pretraživanje na vrhu pronalazi postavku po nazivu ili opisu na bilo kojoj stranici. Svaka promijenjena postavka ima vlastiti gumb za vraćanje zadane vrijednosti, a **Vrati zadane vrijednosti** resetira trenutačnu stranicu.

## Stranice
- **Pregled**: zajedno uobičajene postavke: Izgled (Neprozirnost ploča (%), Shema boja, Raspored, Jezik), Uređivanje, Prikaz i Priručnik.
- **Igre**: igra, gdje su njezine datoteke i vaše ime za suradnju. Igra određuje u kojem se formatu zapisuje mapa, koje se definicije entiteta učitavaju i gdje su njezini materijali. Konfiguracije igara čitaju se iz direktorija `games` u direktoriju konfiguracije (`~/.config/heron-brush/games` na Linuxu i macOS-u, `%LOCALAPPDATA%\heron-brush\games` na Windowsima) i iz direktorija `games` pokraj aplikacije, kamo paketi stavljaju priloženu konfiguraciju.
- **Prikaz**: Raspored, Shema boja, Jezik, osi svijeta, neprozirnost mreže, veličina ćelije za provjeru propuštanja i zaključavanja mapiranja.
- **Boje**: boje prikaza, geometrije i ručki.
- **Miš**: brzine i obrtanja ogledanja, pomicanja pogleda i kretanja, brzina letenja, vidno polje, Poveži 2D prikaze i veličine ručki.
- **Tipkovnica**: raspored tipki, Heron Brush ili Blender.
- **Renderer**: realistično sjenčanje, obris odabira, svjetlo, sjene, ambijentalna okluzija, odrazi, bloom i površine.

## Jezik
**Jezik** nudi Prema sustavu i svaki prevedeni jezik. Sučelje se mijenja odmah. Priručnik se otvara na jeziku sučelja kad postoji njegov prijevod, a inače na engleskom.

# Dodaci i MCP
## Dodaci
Dodaci proširuju uređivač skriptama u jeziku Rhai. Datoteka → Dodaci otvara prozor Dodaci. **Instaliraj dodatak** instalira datoteku `.rhai`, a **Instaliraj direktorij dodatka** instalira direktorij koji sadrži `main.rhai`; neobavezni metapodaci `plugin.toml` opisuju dodatak i njegove postavke. Dodaci su grupirani pod **Aktivni dodaci** i **Neaktivni dodaci**, a **Uključi dodatak** uključuje ili isključuje pojedini dodatak; to ostaje i nakon ponovnog pokretanja. **Ponovno učitaj dodatke** ponovno čita skripte s diska i resetira vrijednosti njihovih panela.

Aktivni dodaci pojavljuju se i u odjeljku **Dodaci** na kartici Mapa sa svojim postavkama, gumbima za radnje (**Pokreni**) i izlazom. Radnja djeluje na odabir ili, kad ništa nije odabrano, na sve, i jedan je korak poništavanja. Priloženi Selection inspector prikazuje brojeve te radnje za pomicanje gore i dolje.

## MCP
Suradnja → Posluživanje za agenta (MCP) omogućuje AI agentu ili drugom alatu da upravlja otvorenim uređivačem putem protokola Model Context Protocol. Prozor prikazuje lokalnu adresu s gumbom **Kopiraj**, a **Zaustavi poslužitelj** ga završava; dok radi, naslovna traka prikazuje **MCP aktivan**. Opcija naredbenog retka `--mcp` pokreće isto sučelje bez prozora. Klijenti mogu otkriti alate za geometriju, resurse, svojstva, animaciju, izvoz i sesije. MCP i dodaci uređivača različita su sučelja.

# Naredbeni redak
Program je `heron-brush`. Bez opcija otvara početni zaslon.

## Otvaranje datoteka
`heron-brush map.map [entities.fgd] [materials/]` otvara mapu, po želji s definicijama entiteta i direktorijem materijala. Prazan argument preskače jedan od njih, primjerice `heron-brush map.map "" materials/`. Ako uređivač već radi, mapa se otvara u njemu; `--new-instance` umjesto toga otvara zaseban prozor.

## Naredbe
- `--help` ili `-h`: ispiši upute za korištenje. `--version` ili `-V`: ispiši verziju.
- `--mcp`: poslužuj MCP putem standardnog ulaza i izlaza, bez prozora.
- `--merge-map <base> <ours> <theirs> [<output>]`: trostruko spajanje datoteka `.map` za git. Izlaz je zadano ours. Sukobi se ispisuju i daju izlazni kôd različit od nule.
- `--obj-to-map <in.obj> <out.map> [size] [thickness] [snap] [material]`: pretvori mrežu u brusheve. Size je duljina najdulje stranice u jedinicama mape (zadano 256), snap je mreža za kutove (0 za bez), a riječi `solid` ili `layered` grade kutije mreže ili konveksne ploče umjesto tankih klinova.
- `--render-preview <in.map> <out.png> [size]`: zapiši PNG sličicu mape, zadano 512 piksela.
- `--run-script <script.rhai> <in.map> [out.map] [key=value ...]`: pokreni skriptu dodatka nad mapom sa svime odabranim i zapiši rezultat (zadano natrag u ulaznu datoteku). `key=value` odgovara na postavke dodatka.

# Rješavanje problema
## Kad nešto izgleda pogrešno
- Materijal nedostaje: provjerite direktorij ili paket materijala i putanje projekta u izvozu.
- Pogrešna veličina modela: usporedite s oznakom igrača od 64 jedinice i mjerilom izvoza.
- Pogrešan smjer prijelaza: provjerite kut prijelaza i orijentaciju plohe.
- Prazna scena u pogonu: pročitajte upozorenja izvoza i slijedite upute za uvoz za cilj.
- Veza uživo čeka: provjerite je li proširenje instalirano, je li scena priključena i jesu li ažuriranja pauzirana.
- Samo linije, bez ploha: isključite Žičani model u odjeljku Prikaz na kartici Mapa ili u izborniku 3D prozora odaberite Materijali ili Puni prikaz.
- Objekti nedostaju u prikazima: `Ctrl+Shift+I` prikazuje skrivene objekte; provjerite slojeve i View filters.
- Tipka ne radi ništa: najprije kliknite u prikaz da dobije tipkovnicu i provjerite raspored tipki u Postavke → Tipkovnica.
- Izmjena nije uspjela: poništite, pojednostavnite geometriju i upotrijebite **Provjeri mapu** i **Provjeri propuštanja** u odjeljku **Problemi** na kartici Mapa.
- Mapa se ne otvara iz upravitelja datoteka: možda je predana uređivaču koji već radi; potražite taj prozor.

## Postavke i sigurnosne kopije
Postavke, nedavne mape, zvjezdice, postavke izvoza i dodaci nalaze se u direktoriju konfiguracije: `~/.config/heron-brush` na Linuxu i macOS-u te `%LOCALAPPDATA%\heron-brush` na Windowsima. Sigurnosne kopije automatskog spremanja nalaze se u direktoriju `autosave` pokraj svake mape.

## Prijava pogreške
Navedite verziju uređivača (Pomoć → O programu), svoj operacijski sustav, malu mapu koja pokazuje problem i točne korake. Okno izlaza (`Ctrl+4`) često kaže što je pošlo po zlu. Prije eksperimentiranja spremite kopiju.

## Korištenje ovog priručnika
Odaberite poglavlje u sadržaju s lijeve strane. `Ctrl+F` prikazuje sadržaj i premješta na polje za pretraživanje; `Esc` briše pretraživanje ili zatvara priručnik kad je pretraživanje prazno. **Sadržaj** skriva ili prikazuje popis poglavlja, a **Veličina teksta** mijenja veličinu fonta; oboje se pamti. Popis jezika na vrhu prebacuje između dostupnih prijevoda.
