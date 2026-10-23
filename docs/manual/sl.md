# Prvi koraki
Heron Brush je urejevalnik stopenj, ki temelji na brushih. Stopnje gradite iz konveksnih brushev, barvate in teksturirate njihove ploskve, postavljate entitete, animirate vrata in ploščadi ter rezultat izvozite v igralni pogon. Na voljo so tudi vgrajena sredstva, parametrične oblike, povezave v živo z urejevalniki pogonov, skupne seje urejanja in upodabljanje statičnih slik. Ta priročnik je priložen programu in deluje brez povezave.

V macOS deluje `Cmd` povsod, kjer priročnik omenja `Ctrl`, Option pa je Alt.

## Pozdravni zaslon
Če se Heron Brush zažene brez zemljevida, se odpre pozdravni zaslon. Na levi so **Nadaljuj** (nazadnje odprti zemljevid), **Nov zemljevid** in **Prebrskaj** za odpiranje obstoječe datoteke `.map`, poleg tega še **Igra**, **Priročnik** in **Nastavitve**. Na desni je seznam nedavnih zemljevidov s sličicami, ki mu sledijo **Primeri**. Zvezdica (**Pripni na vrh**) pripne zemljevid na vrh seznama; koš (**Premakni v koš**) premakne datoteko v sistemski koš, od koder jo je mogoče obnoviti. S puščičnimi tipkami se premikate po zaslonu, Enter pa odpre izbrani element. Datoteka → Dobrodošli pozdravni zaslon kadar koli prikliče nazaj.

## Vaš prvi zemljevid
1. Na pozdravnem zaslonu izberite **Nov zemljevid** ali Datoteka → Nov zemljevid (`Ctrl+N`).
2. V vrstici orodij izberite orodje Brush (`B`) in povlecite v 2D-pogledu, da narišete izbrano obliko. Njeno globino preverite v drugem 2D-pogledu.
3. Izberite brush in ga premaknite z vlečenjem. Na zavihku **Ploskev** mu dodelite material ali barvo.
4. Izvorni zemljevid shranite s `Ctrl+S`. Ko ga želite uporabiti v igri, odprite Datoteka → Izvoz in zapišite sceno za svoj pogon.

## Datoteke in shranjevanje
Datoteka `.map` je vaš urejljivi izvor. Redno shranjujte: izvoz, povezave v živo in upodabljanje zemljevida nikoli ne shranijo namesto vas. Datoteka → Shrani kot (`Ctrl+Shift+S`) doda končnico `.map`, če jo izpustite. Datoteka → Povrni na shranjeni zemljevid zavrže neshranjene spremembe in znova naloži datoteko. Če zaprete okno ali odprete drug zemljevid, medtem ko so spremembe neshranjene, Heron Brush vpraša, ali naj jih **Shrani**, **Zavrzi** ali **Prekliči**.

Dokler ima shranjeni zemljevid neshranjene spremembe, se vsakih deset minut zapišejo oštevilčene varnostne kopije v mapo `autosave` poleg zemljevida. Ohrani se 50 najnovejših kopij.

Če zemljevid odprete, ko Heron Brush že teče, se preda odprtemu oknu, ki pride v ospredje. Če želite ločeno okno, v ukazni vrstici uporabite `--new-instance` (glejte Ukazna vrstica).

# Uporabniški vmesnik
Glavno okno ima naslovno vrstico z meniji, pod njo vrstico orodij, poglede zemljevida, inšpektor na desni in vrstico stanja na dnu.

## Naslovna vrstica in meniji
Naslovna vrstica prikazuje logotip, menije, ime zemljevida na sredini in gumbe okna. V macOS so običajni rdeči, rumeni in zeleni gumbi na levi. Z vlečenjem naslovne vrstice premikate okno, z dvoklikom ga povečate. Ko se nekaj izvaja v ozadju, se proti koncu vrstice prikaže kazalnik: povezava v živo s pogonom, seja v živo ali **MCP je aktiven**. S klikom na kazalnik ustavite povezavo v živo ali odprete njeno okno.

- **Datoteka**: Dobrodošli, Nov zemljevid (`Ctrl+N`), Odpri zemljevid (`Ctrl+O`), Shrani (`Ctrl+S`), Shrani kot (`Ctrl+Shift+S`), Povrni na shranjeni zemljevid, Izvoz, Upodobi sliko, Vtičniki in Nastavitve (`Ctrl+,`).
- **Uredi**: Razveljavi (`Ctrl+Z`), Uveljavi znova (`Ctrl+Shift+Z`), Izreži, Kopiraj, Prilepi, Podvoji (`Ctrl+D`), Izbriši, Izberi vse (`Ctrl+A`), Ponovi zadnje ukaze (`Ctrl+R`) in Pozabi ponovljive ukaze.
- **Pogled**: 3D-pogled v ločenem oknu, Pojdi na, Pripni na mrežo (`Alt+0`), Pokaži mrežo (`0`), Predogled animacij in Pokaži izpuščene ploskve.
- **Geometrija**: Vstavi 3D-besedilo, Spoji kot, Pretvori ploskve v patche, Premakni izbor iz njegove entitete in Sinhroniziraj povezane skupine.
- **Animacija**: Animacija s ključnimi sličicami, Predogled animacij in Postavi kamero po 3D-pogledu (`Ctrl+Alt+0`).
- **Zemljevid**: Naloži, Znova naloži in Zapri datoteko točk; Naloži, Znova naloži in Zapri datoteko portalov. Te prikazujejo sledi puščanj in portale, ki jih zapišejo prevajalniki zemljevidov.
- **Sodelovanje**: Seja v živo in Ponudi agentu (MCP).
- **Pomoč**: Priročnik in O programu.

## Vrstica orodij
Vrstica orodij vsebuje vsa orodja. Prva skupina se uporablja ves čas: Izberi, Brush, Rez, Oglišče, Rob, Ploskev, Zasukaj in Merilo. Druga skupina vsebuje Striži, Sestavi, Izvlek, Kontrolna točka, Zaobljanje in Barvanje. Ko s kazalcem pridržite nad gumbom, se prikažeta njegovo ime in bližnjica. Ponoven klik na aktivno orodje vas vrne na Izberi.

Za orodji sledita **Zrcali vodoravno** in **Zrcali navpično**, nato dve stikali: **Zakleni** ohrani preslikavo materiala ob premiku brusha, **UV** pa jo ohrani ob zasuku brusha (`U` ga preklopi). Sledijo nastavitve aktivnega orodja: orodje Brush prikaže obliko in njene možnosti, orodje Zaobljanje pa polmer, ploskve in kote z gumbom **Uporabi**. Na desnem koncu so ime orodja, velikost mreže (**Mreža 16** in podobno; s klikom izberete drugo) in meni **Nastavitve mreže** z možnostmi Pripni na mrežo, Pokaži mrežo, Predogled animacij, Pokaži izpuščene ploskve in Animacija s ključnimi sličicami.

## Pogledi zemljevida
Na voljo so štirje pogledi: 3D-pogled in trije ravninski pogledi, Zgoraj (XY), Spredaj (XZ) in Od strani (YZ). Nastavitve → Pogled → Postavitev izbere Eno podokno, Dve podokni, Tri podokna ali Štiri podokna. Velikost podoken spremenite z vlečenjem razdelilnikov. Klik v pogled mu preda tipkovnico. `Space` v podoknu, ki prikazuje po en pogled, preklopi na naslednji pogled, `Ctrl+Space` pa poveča pogled, ki ima tipkovnico, in znova obnovi postavitev.

Desni klik brez vlečenja v katerem koli pogledu odpre priročni meni za izbor in za to, kar je pod kazalcem. Ponuja združevanje v skupine, plasti, povezane skupine, ustvarjanje entitet, premikanje brushev v entiteto ali iz nje, kopiranje imena materiala in prikaz materiala v brskalniku materialov.

## Inšpektor
Inšpektor na desni ima štiri zavihke. Njegovo širino spremenite z vlečenjem levega roba.

- **Zemljevid** (`Ctrl+1`): Plasti, Skupine, Vtičniki, Težave, filtri pogleda in Oznake.
- **Entiteta** (`Ctrl+2`): brskalnik entitet, definicije entitet, lastnosti izbrane entitete in pri gumbih in sprožilcih Sprožilec animacije.
- **Ploskev** (`Ctrl+3`): UV, Materiali, Barva, Izvlek, Zaobljanje, Patch ter Vgrez in posnetje. Vsak razdelek je mogoče strniti.
- **Sredstva**: knjižnica modelov iz brushev za večkratno uporabo.

`Ctrl+5` skrije ali prikaže inšpektor, `Ctrl+Alt+T` pa skrije ali prikaže vrstico orodij, kar pogledom da več prostora.

## Vrstica stanja in podokno izpisa
Vrstica stanja na dnu prikazuje zadnje sporočilo urejevalnika. Napake so prikazane rdeče. Klik na vrstico ali `Ctrl+4` odpre podokno izpisa s celotno zgodovino sporočil. Večina ukazov tu sporoči, kaj je naredila, na primer koliko brushev je bilo prerezanih.

## Pogovorna okna in videz
Pogovorna okna, kot so Nastavitve, Izvoz in Upodobi sliko, se odprejo kot samostojna okna. Nastavitve → Pogled → Barvna shema izbere Kot v sistemu, Svetla ali Temna. **Prekrivnost plošč (%)** na strani Pregled naredi plošče nad pogledi prosojne. Okno riše lastne gumbe v slogu vašega sistema: izvorno v macOS, v slogu Windows 11 v sistemu Windows in v slogu GNOME v Linuxu.

# Navigacija in tipkovnica
## Premikanje v 3D-pogledu
Za razgledovanje držite desni gumb miške in vlecite. Z vlečenjem s srednjim gumbom pomikate pogled. Kolesce premika naprej in nazaj. `W`, `A`, `S` in `D` letijo naprej, levo, nazaj in desno, `Q` dviga in `X` spušča; za večje korake držite `Shift`. Če je v Nastavitve → Miška vklopljeno **Alt + vlečenje s srednjim gumbom premika kamero**, Alt s srednjim gumbom premika v smeri pogleda. Hitrosti, obračanje smeri in vidno polje so v Nastavitve → Miška.

## Premikanje v 2D-pogledih
Z vlečenjem s srednjim gumbom pomikate pogled, s kolescem pa povečujete okoli kazalca. Pomikanje in povečava enega ravninskega pogleda ohranita poravnanost ostalih; to nadzira **Poveži 2D-poglede** v Nastavitve → Miška. `Ctrl+Alt+Z` ponastavi povečavo, `Ctrl+Shift+U` pa ravninske poglede vrne v izhodišče.

`Ctrl+U` uokviri izbor v vseh pogledih. Pogled → Pojdi na (`Ctrl+Alt+M`) premakne kamero na vtipkane koordinate, izbere tisto, kar je v določeni vrstici datoteke zemljevida, ali premakne izbor za vtipkano vrednost.

## Mreža in pripenjanje
`+` in `-` spreminjata velikost mreže, številske tipke `1` do `9` pa velikost izberejo neposredno, od 1 do 256 enot. `0` ali `#` prikaže ali skrije mrežo, `Alt+0` pa vklopi ali izklopi pripenjanje. `S` pripne oglišča izbranih brushev na trenutno mrežo, `Ctrl+Shift+V` jih pripne na cele enote, `Ctrl+Alt+Shift+V` pa na mrežo.

## Pogoste bližnjice
- `F3` ali `Ctrl+Shift+P`: poišče katerikoli ukaz menija po imenu in ga zažene z `Enter`. `F10` odpre menije, puščice pa se premikajo po njih.
- `Ctrl+S`: shrani zemljevid. `Ctrl+Shift+S`: shrani kot.
- `Ctrl+Z`: razveljavi. `Ctrl+Shift+Z` ali `Ctrl+Y`: uveljavi znova.
- `Ctrl+C`, `Ctrl+X`, `Ctrl+V`: kopiraj, izreži in prilepi. Lepljenje postavi predmete na mesto kazalca; `Ctrl+Alt+V` ohrani njihove prvotne koordinate. Če so izbrane ploskve, kopiranje prevzame njihov material in nastavitve UV.
- `Ctrl+D`: podvoji. `Ctrl+Shift+D`: povezana kopija skupine.
- `Delete` ali `Backspace`: izbriši izbor.
- `Esc`: prekliče trenutno vlečenje, naredi korak nazaj v aktivnem orodju, se vrne na Izberi in na koncu počisti izbor. `Shift+Esc` se neposredno vrne na Izberi.
- `Ctrl+R`: ponovi zadnje ponovljive ukaze. `Ctrl+Shift+R`: pozabi jih.
- `Ctrl+Alt+0` ali `Ctrl+Alt+Num0`: postavi kamero za upodabljanje na mesto 3D-pogleda.
- `F5`: znova naloži materiale. `F6`: znova naloži definicije entitet.

## Bližnjice za izbiranje
- `Ctrl+A`: izberi vse ali, ko je aktivno orodje za oglišča, robove ali ploskve, vse ročice. `Ctrl+Shift+A`: ne izberi ničesar. `Ctrl+Alt+A`: obrni izbor.
- `Ctrl+B`: izberi druge predmete v isti entiteti ali skupini.
- `Ctrl+T`: izberi vse, česar se izbrani brushi dotikajo. `Ctrl+E`: izberi vse, kar je v njih. `Ctrl+Shift+E`: izberi vse v njihovem stolpcu vzdolž osi pogleda. Pri tem se izbirni brushi porabijo.
- `Ctrl+I`: osami izbor. `Ctrl+Alt+I`: skrij ga. `Ctrl+Shift+I`: znova prikaži vse.
- `Ctrl+G`: združi v skupino. `Ctrl+Shift+G`: razdruži.

## Bližnjice za preoblikovanje
- Puščične tipke ter `Page Up` / `Page Down` premaknejo izbor za en korak mreže v smeri, v katero kažejo v aktivnem pogledu. S `Ctrl` med premikanjem podvajajo, z `Alt` pa sukajo.
- `Alt+X`, `Alt+Y`, `Alt+Z`: zasuk za četrt obrata okoli te osi; z dodanim `Shift` v nasprotno smer.
- `Ctrl+F`: zrcaljenje prek pogleda; `Ctrl+Alt+F`: zrcaljenje navzgor in navzdol. `Ctrl+Shift+X` in `Ctrl+Shift+Y` zrcalita vzdolž X ali Y.
- `Insert`: postavi točkovno entiteto v središče izbora.
- `Tab`: ko je izbran brush, kroži Izberi → Oglišče → Rob → Ploskev → Izberi.

## Razpored bližnjic Blender
Nastavitve → Tipkovnica → Razpored bližnjic ponuja Heron Brush (privzeto, uporabljeno v vsem priročniku) in Blender. Pri Blenderju: `G` izbere in premika, `R` suka, `S` spreminja merilo, `E` izvleče, `1`, `2` in `3` izberejo orodja za oglišča, robove in ploskve, `K` reže, `B` izbira, `A` izbere vse in `Alt+A` počisti izbor, `X` briše, `Shift+D` podvaja, `H` skrije in `Alt+H` prikaže vse, `Ctrl+Shift+Z` pa uveljavi znova. Vse drugo je v obeh enako.

# Brushi in orodja za modeliranje
Brushi so konveksna telesa, določena z ravninami. Zapletene predmete sestavite iz več preprostih brushev, namesto da bi poskušali en brush narediti konkaven. Kjer zadošča preprosta oblika, naj bo brushev čim manj.

## Risanje brushev
Izberite orodje Brush (`B`) in povlecite v pogledu. Vrstica orodij prikaže **Oblika za risanje**: Kvader, Stopnice, Obok, Valj, Stožec, Sferoid (UV), Sferoid (ikozaeder) in Vstavi 3D-besedilo. `Alt+1` do `Alt+7` obliko izberejo neposredno. Glede na obliko vrstica prikaže **Izvotli** in **Debelina:**, za stopnice **Višina stopnice:** in **Smer:**, za okrogle oblike **Os:**, **Stranic:** in način kroga, za krogle **Obročev:** in za ikozaeder **Natančnost:**. Možnosti nastavite pred vlečenjem. Rezultat preverite v več kot enem pogledu.

## Izbiranje
Brush izberete s klikom, s klikom ob `Ctrl` ga dodate v izbor. Vlečenje iz praznega prostora nariše izbirni okvir. V 3D-pogledu klik ob `Shift` izbere posamezno ploskev, klik ob `Ctrl+Shift` pa doda več ploskev. Vlečenje izbranega brusha ga premakne; vlečenje neizbranega izbere in premakne samo tega.

V 3D-pogledu z vlečenjem ploskve izbranega brusha ob `Shift` ploskev izvlečete. Klik ob `Alt` na drugo ploskev nanjo prenese material in poravnavo izbrane ploskve; `Alt+Ctrl` prenese le material, `Alt+Shift` pa zasuk.

## Orodja za oblikovanje
- **Izberi** (`Esc`): izbiranje in premikanje.
- **Oglišče** (`V`), **Rob** (`E`), **Ploskev** (`F`): vlečenje oglišč, robov ali ploskev. Vlečenje ob `Shift` s sredine roba ali ploskve tam doda oglišče. `Delete` odstrani izbrana oglišča.
- **Zasukaj** (`R`), **Merilo** (`T`), **Striži** (`G`): preoblikovanje izbora z ročicami.
- **Rez** (`C`): s klikom na dve ali tri točke določite rezalno ravnino. `Enter` prereže, `Ctrl+Enter` izbere, katero stran ohraniti, `Backspace` ali `Esc` pa odstrani zadnjo točko.
- **Sestavi** (`N`): postavite točke okoli prostornine in pritisnite `Enter`, da iz njih nastane konveksen brush. `Backspace` odstrani zadnjo točko.
- **Izvlek** (`Y`): glejte razdelek o izvleku spodaj.
- **Kontrolna točka** (`P`): preoblikovanje kontrolne mreže patcha.
- **Zaobljanje** (`O`): kliknite rob, da ga zaoblite, ali izberite robove ali oglišča z njihovimi orodji in pritisnite `Enter`. Več ploskev da bolj gladek rezultat in več geometrije.
- **Barvanje** (`K`): glejte Materiali in UV.

## Združevanje brushev
- `Ctrl+J`: združi izbrane brushe v njihovo konveksno ovojnico.
- `Ctrl+K`: odštej izbrane brushe od brushev, ki se jih dotikajo.
- `Ctrl+Shift+K`: izvotli izbor s stenami debeline enega koraka mreže.
- `Ctrl+L`: presek izbora.

## Izvlek, zaobljanje, patch in posnetje
**Izvlek** gradi geometrijo vzdolž poti iz izbranih ploskev. Izberite ploskve (v orodju Izvlek s klikom ob `Alt`), nato v razdelku Izvlek na zavihku Ploskev nastavite **Hod**, **Zasuk** (os in kot), **Merilo**, **Koraki** in **Pot** (poravnava Celoštevilsko ali Prosto; Lok, Ravno ali Esasti zavoj) ter pritisnite **Izvlek**. Vrstica pod kontrolniki pove, koliko brushev bo nastalo.

Razdelek **Zaobljanje** zaobli izbrane robove z nastavitvami **Polmer**, **Ploskve** in **Koti** (Jeralno ali Zaobljeno); ena ploskev naredi posneti rob.

**Patch** prikazuje **Stolpci** in **Vrstice** izbranega patcha z gumbi za dodajanje ali odvzem para. Geometrija → Pretvori ploskve v patche (`Ctrl+P`) iz izbranih ploskev naredi patche.

**Vgrez in posnetje** deluje na izbranih ploskvah: **Vgrez** ploskev pomakne navznoter od njenih robov, **Globina** pa jo potisne navzven (ali navznoter, če je negativna). **Posnemi ploskve** uporabi oboje; tretji gumb doda ploščo ali izreže vdolbino, odvisno od predznaka globine. **Spoji kot** odpre pogovorno okno, ki dva brusha spoji v jeralni ali zaobljeni kot s predogledom.

## 3D-besedilo
Geometrija → Vstavi 3D-besedilo (tudi zadnji element seznama oblik orodja Brush) zgradi brushe iz besedila. Uporablja Heron Block, vgrajeno poligonsko pisavo s posnetimi vogali, sorazmernimi širinami in češkimi naglasi; male črke uporabljajo oblike velikih. Vpišite **Besedilo**, nastavite **Višina** (višina velikih črk; naglasi segajo nad njo) in **Globina**, spremljajte predogled v živo in pritisnite **Vstavi**.

Če operacija ne more ustvariti veljavne geometrije, prilagodite njene mere ali razveljavite in poenostavite obliko.

## Nizi
Geometrija → Niz kopira izbor in prikazuje kopije v zemljevidu, medtem ko spreminjate vrednosti. Števila vključujejo izvirnik, niz pa ustvari največ 1024 kopij.

- **Vrsta** premakne vsako kopijo glede na prejšnjo za **Korak X**, **Korak Y** in **Korak Z** in jo zasuka za **Zasuk na kopijo (stopinje)** okoli izbrane osi.
- **Mreža** zapolni **Število X**, **Število Y** in **Število Z** elementov z razmiki **Korak X**, **Korak Y** in **Korak Z**.
- **Obroč** razporedi elemente znotraj **Kot (stopinje)** okoli središča, oddaljenega **Polmer (enote)** od izbora. Z **Obrni proti središču** se vsaka kopija zasuka z obročem.
- **Razpršitev** postavi kopije na naključna mesta znotraj polmera, prečno na izbrano os. Isto **Seme** da isto razporeditev, **Naključen zasuk** pa zasuka vsako kopijo.

Ko je izbrana ena cela skupina, **Povezane kopije** iz vsake kopije naredi povezan dvojnik, zato sprememba ene doseže vse. **Uporabi** obdrži kopije in jih izbere skupaj z izvirnikom; **Prekliči** jih odstrani. Celoten niz se razveljavi v enem koraku.

## Oblikovanje terena
Teren sestavlja veliko majhnih brushev, katerih površino čopič za oblikovanje potiska ven in vleče noter. Izberite enega ali več brushev in nato Geometrija → Razdeli na stebre, potem pa še obliko stebrov: Trikotnik, Kvadrat ali Šestkotnik. Ukaz jih vzdolž črt mreže razreže na stebre, široke en korak mreže, in stebre postavi v skupino Terrain. Rezi sledijo mreži sveta, zato se sosednji brushi razrežejo na istih mestih in si delijo kote. Vsak steber stoji vzdolž tanke stranice brusha: pokonci v tleh ali stropu, vodoravno v steni. Jama nastane na enak način: izdolbite kvader (Hollow selection) z debelino stene en korak mreže, izberite stene in jih razdelite.

Izbor lahko razdelite tudi v oknu **Oblikovanje**, ki ga odpre gumb na koncu traku za oblikovanje, in sicer na obliko, izbrano v njegovi vrstici Oblika stebrov. Obe mesti si zapomnita nazadnje uporabljeno obliko. **Trikotnik** naredi dva stebra na celico mreže, vrh trikotnika pa ostane raven, ne glede na to, kako ga oblikujete. **Kvadrat** naredi en steber na celico, torej pol manj brushev; njegov vrh se upogne vzdolž diagonale. **Šestkotnik** prekrije brush s šestkotniki, širokimi dva koraka mreže, vsak razdeljen na šest trikotnikov okoli središča, zato vsak vrh ostane raven.

Izberite orodje **Oblikovanje** (`J`) in vlecite po površini v 3D-pogledu. Če držite gumb in miške ne premikate, čopič še naprej dela na istem mestu, desetkrat na sekundo, zato hrib raste, dokler držite. Oranžen obroč prikazuje čopič. Brez izbora čopič spremeni vsak brush pod seboj, z izborom le izbrane. Trak na vrhu okna določa, kaj naredi poteza:

- **Dvigni** in **Spusti** premakneta površino ven ali noter, najbolj v središču obroča in na robu nič. Držite `Shift`, da ju zamenjate.
- **Zgladi** zgladi neravnine.
- **Poravnaj** poravna površino na mesto, kjer se je poteza začela.

V istem oknu **Smer** določa, v katero smer:

- **Gor** premika površino naravnost gor in dol, znotraj kroga na tleh. To je za teren.
- **Kamera** jo premika proti pogledu in stran od njega, znotraj krogle okoli kazalca. Poglejte tla, steno ali strop in čopič oblikuje to.
- **Normala** jo premika ven iz ploskve, nad katero je kazalec, ko se poteza začne.

Dve števili sta **Polmer (enote)** in **Moč (%)**. Premikajo se le koti na strani, ki jo poteza premika; zadnja stran ostane na mestu, steber pa ostane debel vsaj eno enoto. Kjer si brushi delijo kot, ga premaknejo skupaj, zato površina ostane zaprta. Poteza se razveljavi v enem koraku. V 2D-pogledih orodje izbira kot orodje Izbira.

Ko končate z oblikovanjem, izberite teren in nato Geometrija → **Združi stebre** ali gumb v oknu **Oblikovanje**. Sosednji stebri, ki skupaj tvorijo konveksen brush in so videti enako, postanejo en brush, zato ravni deli postanejo nekaj velikih brushev. Površina se ne premakne, združevanje pa razveljavite v enem koraku.

**Združi z ohranitvijo oblike**, takoj zraven v meniju in v oknu, združuje enako, a se ne meni za videz ploskev: kjer dve ploskvi postaneta ena, ta dobi material ene od njiju. Deluje na poljubnih izbranih brushih, ne le na terenu. Štirje kvadri v obliki črke L postanejo dva, nikoli eden, ki bi zapolnil L, celoten oblikovan teren pa se skrči na toliko brushev, kolikor jih najde združevanje sosedov. Združijo se le brushi v isti skupini ali entiteti, zato vrata nikoli ne pogoltnejo stene ob sebi.

Za teren kot v voxel igrah razdelite na stebre **Kvadrat** in v oknu **Oblikovanje** pri **Poteza** izberite **Kocke**. Poteza tedaj premika ves vrh vsakega stebra za cele korake mreže in pusti stranice navpične, zato teren raste v stopnicah. Vsi načini delujejo kot prej; majhne poteze se seštevajo, dokler steber ne zraste za cel korak. Stebri drugih oblik ostanejo nespremenjeni.

**Prosto** ne potrebuje stebrov in deluje na poljubnih brushih. Dvigni dodaja teren v krogli okoli kazalca, Spusti pa ga odvzema, zato lahko teren tvori previse in jame. Zgladi ga izravna, Poravnaj pa ga spravi na raven, kjer se je poteza začela. Vsak dotik zadete brushe na mreži sestavi znova, zaobljene, vse zunaj krogle pa pusti, kot je bilo, tudi ostre robove. Rezultat so navadni brushi, zato na njih delujejo vsa druga orodja, **Združi stebre** pa nato zmanjša njihovo število. Gostejša mreža da bolj gladko površino in več brushev.

# Materiali in UV
## Nalaganje in uporaba materialov
Odprite razdelek **Materiali** na zavihku Ploskev. **Naloži mapo** doda mapo slik, **Naloži .wad** pa paket tekstur; oboje se prikaže pod **Zbirke**, kjer križec element odstrani. Za zožitev seznama tipkajte v **Filtriraj materiale**. S klikom material postane trenutni, **Uporabi za izbor** pa ga nanese na vse izbrane ploskve. **Izberi** ponuja Izberi brushe z njim in Izberi ploskve z njim. **Zamenjaj** zamenja en material z drugim v celotnem zemljevidu ali samo v izboru.

Orodje **Barvanje** (`K`) nanese trenutni material na ploskev pod kazalcem: klik pobarva eno ploskev, dvoklik ves brush, klik ob `Shift` pa material ploskve prevzame kot trenutnega, kot kapalka.

## Urejevalnik UV
Razdelek **UV** na zavihku Ploskev prikazuje material in obris izbrane ploskve. Upravljanje z miško na platnu:
- Kolesce povečuje okoli kazalca. Z vlečenjem z desnim ali srednjim gumbom pomikate pogled. Dvoklik z desnim gumbom ploskev znova uokviri; izbira druge ploskve jo uokviri samodejno.
- Vlečenje z levim gumbom premika material, obris ploskve pa ostane na mestu. Obroč suka preslikavo, ročice v vogalih spreminjajo merilo, središčna ročica pa določa izhodišče teh sprememb.
- Vlečenje z levim gumbom ob `Alt` na robu ploščice striže, enako tudi vlečenje s srednjim gumbom ob `Shift`.
- Vsako vlečenje je en korak razveljavitve, `Esc` pa prekliče vlečenje, ki je v teku.

Pod platnom so **Odmik**, **Merilo**, **Kot** in, kadar jih igra uporablja, **Zastavice vsebine**, **Zastavice površine** in **Vrednost površine**. Gumbi so **Ponastavi**, **Poravnaj po svetu**, **Prilagodi**, **Zrcali vodoravno**, **Zrcali navpično** in zasuka za 90°. **Poravnaj** obrne material vzdolž enega od robov ploskve (ponoven pritisk za naslednji rob, `Shift` za prejšnjega). Štirje gumbi za poravnavo potisnejo material ob rob. **Prilagodi U** in **Prilagodi V** spremenita merilo tako, da ploskev pokrije celo število ponovitev; ponoven pritisk izbere naslednje število, z držanjem `Ctrl` pa ohranite razdelitev trim sheeta.

## Tipke za UV
Ko so izbrane ploskve, puščične tipke premikajo material, `[` in `]` ga zasukata za 15°, `,` in `.` mu razpolovita ali podvojita merilo, `Ctrl+F` in `Ctrl+Alt+F` ga zrcalita, `Shift+R` ga ponastavi, `Alt+Shift+R` ga ponastavi na osi sveta, `Ctrl+Shift+F` pa ga prilagodi vsaki ploskvi.

**Zakleni** v vrstici orodij ohrani preslikavo ob premikanju brushev, **UV** (`U`) pa ob njihovem sukanju.

## Materiali orodij
- skip: označeno ploskev odstrani iz izvozov za pogone.
- clip: ustvari trke brez vidne geometrije.
- origin: označi vrtišče brush entitete.

Imena lahko spremenite na kartici **Materiali orodij** v oknu Izvoz. Skip uporabite za ploskve, za katere veste, da so skrite.

## Izpuščanje ploskev iz izvoza
Izberite ploskev in v razdelku UV vklopite **Izpusti ploskev iz izvoza**. V predogledu UV jo označuje oranžno šrafiranje. Njen material in barva ostaneta shranjena, brush pa ostane zaprt in urejljiv. Izvožene mreže za upodabljanje to ploskev izpustijo, trki pa se ohranijo. Če kocki izpustite en štirikotnik, nastaneta dva trikotnika za upodabljanje. To ploskev ne naredi dvostranskih in ne združuje mrež.

**Izpusti prekrite ploskve** označi ploskve, ki jih statični brushi povsem prekrivajo, vključno z enakimi dotikajočimi se ploskvami in zaprtimi brushi. Deluje na izbranih brushih ali, če ni nič izbrano, na vsej vidni nezaklenjeni geometriji sveta. Brush entitete, prosojni materiali in brushi, ki že imajo izpuščene ploskve, se preskočijo. Delno prekrite ploskve se ne razdelijo. En korak razveljavitve obnovi vse. Pogled → Pokaži izpuščene ploskve take ploskve v pogledih urejevalnika skrije ali prikaže; izbiranje še vedno uporablja ves brush.

# Barva in prelivi
Razdelek **Barva** na zavihku Ploskev barva ploskve z enotnimi barvami ali prelivi prek njihovih materialov. Uporaben je za grobo postavitev stopnje in za stilizirano grafiko. Pobarvane ploskve v pogledih ohranijo svoje prave barve; izbor je prikazan z obrobo.

## Paleta in mešana barva
Kliknite enega od dvanajstih vzorcev palete, da z njim pobarvate izbrane ploskve ali cele izbrane brushe. **Prekrivnost** v odstotkih določa prekrivnost naslednjega barvanja. **Mešaj** ima barvni gumb, ki odpre izbirnik barv; vlecite v izbirniku in izbor sproti sledi, ali pritisnite **Barvanje**, da mešano barvo nanesete kot en korak. **Nazaj na material** odstrani barvo, tako da ploskev spet prikazuje svoj material. Vrstica na vrhu razdelka pove, kaj je izbrano in kaj nosi.

## Izbirnik barv
Vsak barvni gumb v urejevalniku odpre isti izbirnik. Z vlečenjem v kvadratu izberete nasičenost in svetlost, v traku ob njem pa odtenek. Spodaj so vrednosti **R**, **G** in **B** ter drsnik **Prekrivnost**, kjer ga barva ima. Šestnajstiško polje sprejme `#RRGGBB` ali `#RRGGBBAA`; obarva se rdeče, dokler besedilo ni veljavna barva. **Kopiraj** šestnajstiško kodo kopira v odložišče. Izbirnik zaprete s klikom zunaj njega.

## Prelivi
**Preliv** izbere Linearni preliv ali Radialni preliv, gumb ob njem pa barve zamenja od konca do konca. Vsaka točka ima barvni gumb in **Položaj (%)**, točke med konci pa imajo gumb za odstranitev. **Dodaj barvno točko** doda točko v največjo vrzel; preliv ima lahko največ osem točk. **Kot preliva** določa smer linearnega preliva, z gumboma **+45°** in **−45°**.

Pod predogledom je tračnica z oznako za vsako točko. Klik na tračnico doda točko, klik na oznako odpre njen izbirnik, vlečenje oznake jo premakne, desni klik na oznako pa jo odstrani. Trije vzorci pod tračnico so prednastavitve: Topel preliv, Hladen preliv in Sivinski preliv. Spremembe preliva se na izboru uveljavijo sproti.

Smer preliva je pomembna: usmerite ga vzdolž naklona strehe ali vzdolž ukrivljenega predmeta, ne prek nepovezanih ploskev. Po barvanju model preverite z obeh strani.

## Material površine
**Material površine** nastavi **Hrapavost** (0 je sijajno, 100 je mat), **Kovinskost** (0 za les, kamen ali plastiko, 100 za kovino) in **Emisija** (0 do 8; površina sveti v lastni barvi, vključno s prelivi, vendar ne osvetljuje bližnjih predmetov). Spremembe se sproti uveljavijo na izbranih ploskvah ali brushih. **Uporabi privzeto površino** odstrani prilagoditev. Vse spremembe je mogoče razveljaviti.

Učinki površine so vidni v osvetljenem 3D-pogledu in v upodobitvah kamere. Ploski prikaz in žični model namenoma izpustita odsevne poudarke. Bloom lahko okoli emisivnih površin doda sij. Godot in glTF/GLB prejmeta vrednosti materiala, uvoznik za Unity pa hrapavost pretvori v gladkost. Omejitve posameznih ciljev so opisane v `docs/surface-materials.md`.

# Sredstva, plasti in skupine
## Knjižnica sredstev
Odprite zavihek **Sredstva**. Mapo z modeli `.map` dodate z **Dodaj mapo**; vgrajena knjižnica je vedno na voljo. Za iskanje tipkajte v **Filtriraj sredstva**. S klikom model izberete, nato ga z dvoklikom, s pritiskom `Enter` ali s klikom **Vstavi** dodate tja, kjer je bil nazadnje kazalec. Model lahko tudi povlečete v pogled. Brushi sredstva se vstavijo kot ena skupina, tako da se model premika kot celota.

**Shrani izbor kot sredstvo** zapiše izbrane brushe v knjižnico kot `.map`. Sredstva iz knjižnice so običajni urejljivi brushi, ne uvožene mreže.

## Plasti
Seznam **Plasti** na zavihku Zemljevid organizira velika območja. Vpišite ime in kliknite **Dodaj**, da ustvarite plast, ali pritisnite `Enter`, da preimenujete trenutno. Vsako plast je mogoče skriti, zakleniti ali izpustiti iz izvozov. **Premakni sem** premakne izbor v trenutno plast. Priročni meni v pogledih ponuja tudi Premakni v plast, Nastavi kot aktivno plast, Skrij plasti in Osami plasti.

## Skupine in povezane skupine
`Ctrl+G` izbor združi v skupino, `Ctrl+Shift+G` pa ga razdruži. Razdelek **Skupine** na zavihku Zemljevid navaja skupine in ponuja **Odpri**, **Preimenuj**, **Združi**, **Razdruži**, **Vzemi ven** in **Izloči**. Povezana skupina je niz kopij, ki si delijo spremembe: `Ctrl+Shift+D` naredi povezano kopijo, Geometrija → Sinhroniziraj povezane skupine posodobi kopije, **Prekini povezavo** pa kopijo osamosvoji. Na zavihku Entiteta lahko lastnost ostane lastna posamezni kopiji, ko se povezane skupine sinhronizirajo.

## Skupno merilo
Privzeto merilo izvoza je 32 enot zemljevida na meter. Vgrajena oznaka začetka igralca je visoka 64 enot, kar je 2-metrska referenca. Z njo primerjajte vrata, vozila, pohištvo in rastline. Merilo izvoza spreminjajte le namerno in povezana sredstva ohranjajte usklajena.

# Entitete in igranje
Pogoste definicije entitet so vgrajene in vedno na voljo: luči, začetki igralca, vrata, gumbi, ploščadi, sprožilci, statični rekviziti, zvoki in točke poti. Nalaganje FGD projekta z **Definicije entitet** na zavihku Entiteta zamenja razrede z enakim imenom, ostale vgrajene pa ohrani.

## Postavljanje entitet
V brskalniku **Entitete** na zavihku Entiteta tipkajte v **Filtriraj razrede** in izberite razred. Pri točkovnem razredu, na primer luči, kliknite **Postavi** ali dvokliknite razred; `Insert` ga postavi v središče izbora. Pri brush razredu, na primer vratih ali sprožilcu, izberite brushe in kliknite **Ustvari**. Razrede lahko tudi povlečete v pogled.

## Lastnosti
Seznam lastnosti ureja izbrano entiteto. Entitete istega razreda se urejajo skupaj. **Dodaj lastnost** doda ključ, ki ga definicija morda ne pozna; **Privzeto** lahko obstoječe lastnosti vrne na privzete vrednosti ali doda manjkajoče. Lastnosti, ki jih razred definira, entiteta pa jih nima, so prikazane zatemnjeno; z vpisom vrednosti jih dodate. Za povezovanje entitet uporabite targetname in target.

## Kaj zmore izvoz
Direktive za Godot ustvarijo izvorna vozlišča, telesa trkov in območja sprožilcev. Unity in Unreal podpirata izvorne pogoste luči, Unreal pa ustvari tudi akterje PlayerStart. Drugi podatki se ohranijo, kolikor to dopušča uvoznik cilja.

Entiteta vrat sama ne doda vedenja v igri. Gibanje, odzivi sprožilcev, krmilniki igralca in skripti projekta še vedno zahtevajo delo v pogonu. Defold izvozi lastnosti entitet in objekte trkov; Flax postavi modele in splošne entitete. Rezultat preverite v svojem pogonu, preden nanj gradite igranje.

# Animacija
Heron Brush omogoča predogled preprostega gibanja in urejanje animacij brush entitet s ključnimi sličicami ter njihov izvoz v več pogonov.

## Predogled animacij
Vklopite **Predogled animacij** v meniju Pogled, meniju Animacija, meniju Nastavitve mreže ali v razdelku Pogled na zavihku Zemljevid. Predvaja vrtenje func_rotate in gibanje sem ter tja func_shuttle, kadar je omogočen start_on, pri čemer uporablja lastnosti entitete za os, hitrost, smer, hod in čakanje, ter predvaja posnetke s ključnimi sličicami, ki imajo nastavljeno Predvajaj ob zagonu. Sredstvi ventilator in mlin na veter že vsebujeta vrteče se dele.

Izklop predogleda obnovi prvotno pozo. Klik v pogled ustavi predvajanje pred izbiranjem ali urejanjem; navigacija s kamero deluje naprej. Predogled ne spreminja zemljevida, zgodovine razveljavitev ali izvozov in ne izvaja fizike ali logike igre.

## Urejevalnik ključnih sličic
Odprete ga z Animacija → Animacija s ključnimi sličicami ali iz menija Nastavitve mreže. Zasidra se pod pogledi ali v 3D-oknu, kadar je to ločeno. Ikona okna v njegovem naslovu ga premakne v samostojno okno; pri tem ohrani sličico in zgodovino razveljavitev. **Uredi animacijo** preklopi poglede na animirano pozo, **Uredi zemljevid** pa se vrne na geometrijo v mirovanju. V načinu animacije vsak pogled na dnu prikazuje **Uredi animacijo** in številko sličice.

Za animiranje izberite brush entiteto ali brush ali skupino brushev. Ploska skupina brushev ima eno animacijo in vrtišče; skupine, ki vsebujejo več entitet, potrebujejo ločene animacije.

## Posnetki
Izbirnik **Animacija** preklaplja med poimenovanimi posnetki. Vpišite ime v **Ime animacije** in kliknite **Shrani kot**, da trenutni posnetek kopirate pod tem imenom; **Izbriši** posnetek odstrani. **Premik** ustvari posnetek `translate` brez zasuka. **Animacije vrat** ustvari gladka posnetka `open` in `close` z zasukom za 90 stopinj okoli trenutnega vrtišča; vrtišče najprej premaknite na tečaj. **Predvajaj ob zagonu** poskrbi, da se posnetek predvaja ob začetku stopnje; prednastavitev za vrata to izklopi.

## Upravljanje in predvajanje
Gumbi za upravljanje so **Začetek**, **Prejšnji ključ**, **Predvajanje**, **Naslednji ključ** in **Konec**. **Sličica** prikazuje in nastavlja trenutno sličico, **Trajanje (s)** pa dolžino posnetka. Način je **Enkrat**, **Zanka** ali **Tja in nazaj**.

## Časovnica
Časovnica ima ravnilo s predvajalno glavo in pas za **Položaj** (X, Y in Z se zapisujejo skupaj) ter za vsak kot zasuka, RX, RY in RZ. S klikom ali vlečenjem v ravnilu premaknete predvajalno glavo in v vseh pogledih vidite predogled te poze. Z vlečenjem s srednjim gumbom pomikate časovnico. Ključe v času premikate z vlečenjem; `Shift` dodaja v izbor; dvoklik na pas vstavi ključ; `Ctrl`+kolesce povečuje. **Krivulje** namesto pasov prikaže krivulje vrednosti.

Gumbi nad njo so **Dodaj ključ**, **Odstrani**, **Kopiraj**, **Prilepi** in **Podvoji**. **Pripni na sličice** ohranja ključe na celih sličicah, **Prilagodi** pa prikaže ves posnetek. **Spremeni časovanje ključev** odpre **Časovno merilo**, **Odmik (sličice)**, **Merilo od predvajalne glave**, **Podaljšaj trajanje** in **Spremeni časovanje izbora**.

Inšpektor ključev ob časovnici ureja izbrani ključ: **Sličica**, **Vrednost** (ali tri koordinate ključa položaja), interpolacijo **Linearno**, **Konstantno** ali **Bézierjeva**, **Vhodni naklon** in **Izhodni naklon** ter **Zgladi tangente**.

## Tipke v urejevalniku
Ko ima časovnica tipkovnico: `Space` predvaja ali ustavi, leva in desna puščica se premakneta za eno sličico, `Ctrl+A` izbere vse ključe, `Ctrl+C`, `Ctrl+X` in `Ctrl+V` kopirajo, izrežejo in prilepijo ključe, `Ctrl+D` jih podvoji, `Delete` jih odstrani, `Ctrl+Z` razveljavi, `Esc` pa prekliče vlečenje. Zunaj časovnice `Esc` zapusti način animacije. Razveljavitev pusti povečavo, izbor in predvajalno glavo tam, kjer so bili.

## Poziranje v pogledih
Vrstica pod časovnico vsebuje **Prikaz sosednjih sličic (onion skin)** s številom in **Razmik**, ki bližnje poze prikaže kot duhove; koordinate **Vrtišče**; **Samodejni ključi**; ter orodja za poziranje **Premik**, **Zasukaj** in **Vrtišče**.

- **Dodaj ključ** vklopi Samodejni ključi in ustavi predvajanje. Ko so Samodejni ključi vklopljeni in je izbran **Premik**, v 3D-pogledu povlecite obarvano os ročice izbrane entitete, da zapišete ključ njenega položaja na trenutni sličici.
- V 2D-pogledu povlecite izbrani brush ali člana njegove skupine, da zapišete ključ premika vzdolž obeh vidnih osi s pripenjanjem na mrežo. Samodejni ključi za to ne rabijo biti vklopljeni.
- Z **Zasukaj** povlecite brush v 2D-pogledu okoli njegove oranžne oznake vrtišča. Zgoraj zapiše RZ, Spredaj RY in Od strani RX, pripeto na kote mreže.
- Z **Vrtišče** povlecite rumeno središče vrtenja v katerem koli 2D-pogledu. Pripne se na mrežo in ohrani svojo globino. Ključi in geometrija v mirovanju ostanejo nespremenjeni.

Cijan pot v 2D in 3D prikazuje gibanje vrtišča. Klik na točko poti izbere njen čas in ključe, vlečenje pa spremeni položaj; ko so Samodejni ključi vklopljeni, lahko točke poti vlečete tudi v 3D-pogledu. Oznake prikazujejo številke sličic, črtkani odseki pa označujejo skoke. `Esc` prekliče katero koli od teh vlečenj, en korak razveljavitve pa povrne celotno vlečenje.

Nove animacije se začnejo v mirovanju s ključi v času nič. Gibanje ustvarite tako, da na drugi sličici spremenite pozo. Za oster pot v obliki črke L zapišite ključe začetka, vogala in konca ter uporabite Linearno s Tja in nazaj.

## Povezovanje gumbov z animacijami
Izberite func_button, trigger_multiple ali trigger_once. Zavihek Entiteta nad lastnostmi prikaže **Sprožilec animacije**. Izberite cilj s seznama ali kliknite **Izberi cilj v pogledu** in nato animiran predmet v katerem koli pogledu; `Esc` prekliče izbiranje. Manjkajoči cilji ali posnetki so sporočeni pod kontrolniki. Povezave entitet prikazujejo puščice do svojih ciljev, kadar je prikaz povezav vklopljen.

**Dejanje** izbere, kaj naredi aktivacija: **Predvajaj** zažene poimenovani posnetek ali izbrano animacijo; **Preklopi** izmenjuje `open` in `close`, če obstajata, sicer znova zažene izbrano animacijo; **Odpri** in **Zapri** zaženeta ta posnetka; **Premor** zadrži pozo, **Nadaljuj** nadaljuje, **Ustavi** pa obnovi prvotno pozo. **Samo enkrat** preglasi privzeto vrednost razreda. **Zakasnitev ponavljanja (s)** določa najkrajši čas med aktivacijami; prve ne zakasni.

**Preizkusi sprožilec** izvede dejanje v predogledu, ne da bi urejal zemljevid ali dodajal korake razveljavitve. **Ponastavi predogled** obnovi prvotne poze in počisti stanje enkratnosti in zakasnitve. To ne simulira trkov igralca ali skriptov igre.

## Animacija v izvozih
Godot izvozi izvorna vira AnimationPlayer in AnimationLibrary, vključno s krivuljami in premikajočimi se trki. CharacterBody3D, ki vstopi v območje gumba, aktivira njegov cilj. Spremembe teh posnetkov v Godotu se ne vrnejo v zemljevid in jih naslednji izvoz zamenja.

Izvoz za raylib doda `<map>_animation.h` s predvajalnikom v C in `<map>_preview.c` s predogledom s prosto kamero; za gradnjo sledite `<map>_raylib.md`. V predogledu `SPACE` ustavi, `R` znova zažene privzete posnetke, `N` pa kroži med poimenovanimi posnetki. Defold vgradi predvajanje v skript entitete z dokumentiranimi sporočili `heron_*_animation`. Build Level v Unityju doda `HeronBrushAnimation` in kinematično telo. Izvoz za Bevy vključuje `heron_animation.rs` in navodila. Godot, Unity, Defold, Bevy in raylib ohranijo povezave sprožilcev, zastavico enkratnosti in zakasnitev ponavljanja. Vaša igra mora še vedno registrirati predvajalnik in povezati lastne gumbe z API-jem posameznega izvajalnega okolja.

## Primer Motion lab
Med Primeri na pozdravnem zaslonu odprite `motion_lab`. Vsebuje ploščad s potjo v obliki črke L, dvigalo, vrtečo se ploščad in vrata na tečajih z gumbi. V Godotu odprite `godot/project.godot`, ki je poleg njega, in pritisnite `F5`. WASD in miška premikata igralca, preslednica skoči, 1 do 4 obiščejo postaje, R pa znova zažene trenutno postajo. Stopite na obarvano ploščico, da aktivirate njen cilj. Priložena datoteka README razloži ponovni izvoz.

# Izvoz v pogon
Datoteka → Izvoz odpre okno Izvoz. Pogoni so navedeni na levi z logotipi: Godot, Defold, three.js, Babylon.js, PlayCanvas, Bevy, raylib, Unity, Unreal Engine, Flax Engine, Cocos Creator, Wavefront .obj, glTF .gltf, GLB .glb in na koncu `.map`. Stran na desni pove, kaj bo cilj prejel. **Zapiši** ga zapiše, **Preveri** zgradi stopnjo, ne da bi jo zapisal, in navede težave, **Zapri** pa zapre okno.

Izvoz je ločen od shranjevanja: `.map` ostane vaš urejljivi izvor. Vsak kontrolnik shrani nastavitve svojega pogona takoj, ko se spremeni, zato vsak pogon ohrani svoje odgovore in okno se odpre tako, kot ste ga pustili.

## Skupne možnosti
- **Merilo**: **Enot na meter**, privzeto 32.
- **Kaj zapisati**: **Geometrija trkov**.
- **Optimizacije**: **Združi statične mreže v celice po 1024 enot**, **Odstrani ploskve, zakopane v geometrijo**, **Združi ploskve v isti ravnini** in **Združi oblike trkov**.
- **Poti projekta**: kam znotraj projekta gredo stopnja in njeni viri.
- **Materiali orodij**: imena materialov clip, skip in origin.
- **Več**: stikala, specifična za pogon, kot so **Poimenuj vozlišča po**, **Obnovi plasti in skupine kot nadrejena vozlišča**, **Obrni vrstni red oglišč trikotnikov**, zrcaljenje teksturnih koordinat in drugi nabor UV za zapečeno svetlobo s **Teksel svetlobne karte**.
- **Sprotno sledi zemljevidu**: **Znova zapiši ob vsakem shranjevanju zemljevida**. Na voljo je, ko je bil pogon enkrat izvožen, saj ta izvoz pove, kam pisati.

Statične celice brushe sveta razporedijo po njihovih središčih in jih znotraj vsake celice velikosti 1024 enot združijo po materialu, tako da lahko pogoni izločajo manjše dele zemljevida. Trki se ohranijo, entitete igranja pa ostanejo ločene. To lahko pomeni več klicev risanja kot ena mreža sveta, zato je privzeto izklopljeno. Samostojni izvozi OBJ, glTF in GLB celic ne uporabljajo.

## Opombe o ciljih
- Godot: poti `res://` za teksture in materiale, fizikalna plast sprožilcev in UV2 za svetlobne karte. Izberite mapo za izvoz znotraj projekta Godot.
- Defold: mapa virov sredstev, material modela in vzorčevalnik tekstur, ki mora obstajati v izbranem materialu.
- Unity: mreže OBJ in uvoznik za urejevalnik; izvozite v mapo Assets projekta.
- Unreal Engine: ustvarjen uvozni skript in pot vsebine, običajno /Game/<level>.
- Flax Engine: pot vsebine modela glede na mapo Content projekta.
- three.js, Babylon.js, PlayCanvas, Bevy, raylib in Cocos Creator: scene ali koda za ta izvajalna okolja z navodili poleg izvoza.
- glTF / GLB: prenosljiva geometrija za združljiva orodja; GLB je binarna oblika. OBJ: geometrija in materiali za orodja za modeliranje.
- `.map`: **Zapiši kot** izbere Valve 220, Standardno, Quake 2 ali Quake 2 (Valve). Okno pove, kaj bi izbrana različica izgubila.

Po zapisu preberite opozorila, na primer o manjkajočih teksturah. **Kaj je preverjanje odkrilo** jih navede v oknu.

# Povezave v živo: Godot, Unity, Defold in Flax
Povezava v živo med delom pošilja neshranjene spremembe iz Heron Brusha v urejevalnik pogona. Je enosmerna: spremembe v pogonu se ne vrnejo. Na okno zemljevida lahko teče ena povezava v živo. Pred menjavo pogona jo ustavite, za uveljavitev spremenjenih možnosti izvoza pa jo znova zaženite. Kontrolniki so v oknu Izvoz, na strani pogona, na kartici **Povezava v živo z**.

Kazalnik v naslovni vrstici prikazuje stanje povezave; s klikom nanj povezavo ustavite. Zaprtje ali zamenjava zemljevida ustavi objavljanje. Po neuspelem uvozu odpravite težavo in povezavo znova zaženite.

## Godot
1. Enkrat izvozite v projekt Godot.
2. Kliknite **Namesti dodatek za Godot**.
3. V Godotu omogočite Heron Brush Live v Project Settings → Plugins.
4. Kliknite **Zaženi povezavo v živo** in iz sidrišča Godot zemljevid pripnite odprti 3D-sceni.

Dodatek zamenja le svojo ustvarjeno vejo ter ohrani postavitev pripetka in druga vozlišča. Sceno Godot shranite sami, lastne skripte in vozlišča pa hranite zunaj ustvarjene veje.

## Unity
1. Enkrat izvozite v mapo Assets projekta Unity.
2. Kliknite **Namesti dodatek za Unity** in pustite, da ga Unity prevede.
3. V Unityju odprite Window → Heron Brush → Live Link.
4. Zaženite povezavo in zemljevid pripnite aktivni sceni Unity.

Okno povezave v živo v Unityju naj ostane odprto. Posodobitve se v Play Mode in Prefab Mode ustavijo. Zamenja se le podrejeni objekt Generated objekta zemljevida. Ustvarjenih datotek ne urejajte.

## Defold
1. Enkrat izvozite v svoj projekt Defold in projekt pustite odprt v Defoldu.
2. Kliknite **Zaženi povezavo v živo**. Dodatek ni potreben.
3. V Defoldu odprite heron_brush_live/preview.collection ali se nanj sklicujte iz lastne scene.

Povezava piše v ločeno ustvarjeno mapo, končnega izvoza in game.project pa se ne dotika. **Sprotno osveži zagnano igro** posodobi tudi igro, zagnano iz Defolda; dodajanje objektov ali spreminjanje trkov lahko zahteva ponoven zagon stopnje. Neuspele povezave se samodejno ponovijo.

## Flax
Izvozite v mapo Content projekta in kliknite **Namesti dodatek za Flax**. Pustite, da ga Flax prevede, zaženite povezavo in nato v odprti sceni Flax izberite Tools → Heron Brush: attach live map. Ustvarjeni koren ohrani svojo postavitev, drugi objekti ostanejo nedotaknjeni. Sceno shranite sami. Posodobitve se v Play Mode ustavijo. Tools → Heron Brush: stop live map ustavi sprejemanje.

# Skupne seje v živo
Sodelovanje → Seja v živo odpre okno za gostovanje ali pridružitev drugemu uporabniku Heron Brusha. Tako si urejanje zemljevida deli več ljudi; to je ločeno od povezave v živo s pogonom.

## Gostovanje in pridružitev
Vpišite **Vaše ime**. Za gostovanje izberite **Gosti sejo**, kliknite **Začni sejo** in gostom pošljite podatke iz **Kopiraj podatke za povezavo**. Za pridružitev izberite **Pridruži se seji**, prilepite podatke in kliknite **Pridruži se**. Pridružitev zamenja odprti zemljevid z gostiteljevim, zato najprej shranite svoje delo.

Standardna različica deluje v lokalnem omrežju, **Vrata lokalnega omrežja** pa določajo vrata. Seje prek interneta (**Poveži prek interneta**) zahtevajo različico z izbirno omrežno funkcijo. Udeleženci v 3D-pogledu vidijo oznake kamer in imena drug drugega.

Zaprtje okna sejo pusti teči; kazalnik v naslovni vrstici prikazuje njeno stanje. **Ustavi sejo** ali **Zapusti sejo** jo konča. Ko dva človeka urejata isti predmet, o končni različici odloča gostiteljev vrstni red, tudi razveljavitev pa je skupna sprememba. Kjer je mogoče, delajte na ločenih območjih in shranjujte kontrolne točke.

# 3D-prikaz in drugi zaslon
## Načini prikaza
Razdelek **Pogled** na zavihku Zemljevid preklaplja, kaj pogledi rišejo: Brushi, Točkovne entitete, Modeli entitet, Imena razredov entitet, Obrobe skupin, Patchi, Materiali, Pokaži izpuščene ploskve, Predogled animacij, Žični model, Robovi in Obrobe brush entitet. Žični model riše robove trikotnikov, vključno z diagonalami in robovi za površinami, brez materialov. To je začasen pogled za odpravljanje napak in ne vpliva na izvoze ali upodobitve.

Osvetlitev, sence, ambientna zastrtost, odsevi, bloom in glajenje za 3D-pogled so v Nastavitve → Upodabljalnik. To so nastavitve predogleda; končne slike uporabljajo lastne nastavitve kamere za upodabljanje.

## Ločeno 3D-okno
Pogled → 3D-pogled v ločenem oknu premakne 3D-pogled v samostojno okno, ki ga lahko postavite na drug zaslon. Deli si kamero, dokument in orodja, ravninski pogledi pa zasedejo prostor v glavnem oknu. Njegov meni ponuja Materiali, Polni prikaz in Žični model; podmeni Pogled s Pokaži izpuščene ploskve, Predogled animacij, Robovi, Obrobe skupin, Točkovne entitete, Modeli entitet in Obrobe brush entitet; Izostri na izbor; Spredaj, Od strani in Zgoraj, ki kamero obrnejo okoli izbora (ali točke pred kamero); Postavi kamero po 3D-pogledu; Celozaslonski način (`F11`); Upodobi sliko; in Vrni v glavno okno. Zaprtje okna pogled znova zasidra.

## Proceduralno nebo
Proceduralno nebo je v 3D-pogledu privzeto vklopljeno in ga je mogoče izklopiti v Nastavitve → Upodabljalnik → Luč. Kamere za upodabljanje imajo lastno stikalo **Proceduralno nebo**, ki se shrani z zemljevidom. Barve neba, obzorja in tal prihajajo iz nastavitev okolja, sonce pa sledi osvetlitvi scene. Nebo nima geometrije in se ne izvaža.

# Kamere za upodabljanje in slike PNG
Datoteka → Upodobi sliko odpre okno za upodabljanje: kamera in njene nastavitve so na levi, na desni pa predogled v živo, ki se ob spremembah znova izriše. Kamere se shranijo v `.map` v plasti kamer za upodabljanje, ki je namenjena samo urejevalniku in se izpusti iz izvozov za pogone.

## Postavljanje kamere
Kader uokvirite v 3D-pogledu in pritisnite `Ctrl+Alt+0` (ali `Ctrl+Alt+Num0`) ali kliknite **Postavi kamero po 3D-pogledu**. Če ni izbrana nobena kamera, se posodobi prva shranjena; če ne obstaja nobena, se ustvari nova. To je mogoče razveljaviti in ohrani nastavitve izhoda in osvetlitve. **Izberi kamero v zemljevidu**, **Naloži kamero iz zemljevida** in **Premakni 3D-pogled h kameri** delujejo v nasprotni smeri. **Okvir kamere v 3D-pogledu** prikaže meje izhoda, **Upravljaj kamero iz 3D-pogleda** pa omogoči, da običajna navigacija premika shranjeno kamero.

## Nastavitve
- **Položaj kamere**: X, Y, Z, odklon in naklon.
- **Izhod in objektiv**: **Prednastavitve** (Full HD, Kvadrat, Pokončno), širina, višina, vzorci, **Navpično vidno polje** in **Daljna rezalna ravnina**.
- **Osvetlitev**: Dnevna svetloba, Sončni zahod ali Oblačno, ali pa nastavite osvetlitev, sonce, barve neba, obzorja in tal, **Proceduralno nebo**, **Sence**, **Ambientna zastrtost** in **Bloom**.
- **Globinska ostrina**: **Vklopi globinsko ostrino**, razdalja izostritve, ostro območje in polmer zamegljenosti. **Izostri na izbor** izostri na središče izbranega predmeta pred kamero.

**Shrani kamero in nastavitve** shrani obrazec v zemljevid; za trajno shranitev na disk shranite zemljevid.

## Upodabljanje
**Upodobi PNG** vpraša, kam shraniti, in zažene postopno upodabljanje, ki se v predogledu dopolnjuje vzorec za vzorcem. Več vzorcev da lepši rezultat, a traja dlje. **Prekliči upodabljanje** ustavi brez zapisa datoteke. Upodabljalnik uporablja rastrski cevovod urejevalnika, ne sledenja poti. PNG ima neprosojno ozadje, animacije pa se ne upodabljajo. Zelo velike slike lahko urejevalnik za trenutek zaustavijo.

# Nastavitve
Datoteka → Nastavitve (`Ctrl+,`) odpre nastavitve. Spremembe se uveljavijo takoj in se shranijo. Stranska vrstica navaja Pregled, Igre, Pogled, Barve, Miška, Tipkovnica in Upodabljalnik. Iskalno polje na vrhu najde nastavitev po imenu ali opisu na kateri koli strani. Vsaka spremenjena nastavitev ima lasten gumb za obnovitev privzete vrednosti, **Obnovi privzeto** pa ponastavi trenutno stran.

## Strani
- **Pregled**: skupne nastavitve na enem mestu: Videz (Prekrivnost plošč (%), Barvna shema, Postavitev, Jezik), Urejanje, Pogled in Priročnik.
- **Igre**: igra, kje so njene datoteke, in vaše ime za sodelovanje. Igra določa, katero obliko zemljevida zapisati, katere definicije entitet naložiti in kje so njeni materiali. Konfiguracije iger se berejo iz mape `games` v mapi s konfiguracijo (`~/.config/heron-brush/games` v Linuxu in macOS, `%LOCALAPPDATA%\heron-brush\games` v sistemu Windows) in iz mape `games` poleg aplikacije, kamor paketi shranijo priloženo konfiguracijo. Če v mapi igre ni mape, ki jo navaja konfiguracija igre, na primer `baseq3` za Quake 3 v namestitvi OpenArene, Heron uporabi nadomestek, ki ga konfiguracija pozna (`baseoa`), sicer edino mapo tam s paketi `.pak` ali `.pk3`, in to sporoči pod **Modifikacije**. Mape, navedene v **Modifikacije**, se še vedno priklopijo nad njo.
- **Pogled**: Postavitev, Barvna shema, Jezik, osi sveta, prekrivnost mreže, velikost celice za preverjanje puščanj in zaklepi preslikave.
- **Barve**: barve pogledov, geometrije in ročic.
- **Miška**: hitrosti in obračanje smeri razgledovanja, pomikanja in premikanja, hitrost letenja, vidno polje, Poveži 2D-poglede in velikosti ročic.
- **Tipkovnica**: razpored bližnjic, Heron Brush ali Blender.
- **Upodabljalnik**: realistično senčenje, obroba izbora, luč, sence, ambientna zastrtost, odsevi, bloom in površine.

## Jezik
**Jezik** ponuja Kot v sistemu in vse prevedene jezike. Vmesnik se spremeni takoj. Priročnik se odpre v jeziku vmesnika, če zanj obstaja prevod, sicer v angleščini.

# Vtičniki in MCP
## Vtičniki
Vtičniki razširjajo urejevalnik s skripti Rhai. Datoteka → Vtičniki odpre okno Vtičniki. **Namesti vtičnik** namesti datoteko `.rhai`, **Namesti mapo vtičnika** pa mapo, ki vsebuje `main.rhai`; izbirni metapodatki `plugin.toml` opisujejo vtičnik in njegove nastavitve. Vtičniki so razvrščeni pod **Aktivni vtičniki** in **Neaktivni vtičniki**, **Vklopi vtičnik** pa posameznega vklopi ali izklopi; to se ohrani tudi po ponovnem zagonu. **Znova naloži vtičnike** skripte znova prebere z diska in ponastavi vrednosti v njihovih ploščah.

Aktivni vtičniki se prikažejo tudi v razdelku **Vtičniki** na zavihku Zemljevid z nastavitvami, gumbi za dejanja (**Zaženi**) in izpisom. Dejanje deluje na izboru ali, če ni nič izbrano, na vsem in je en korak razveljavitve. Priloženi inšpektor izbora prikazuje števila ter dejanja za premik navzgor in navzdol.

## MCP
Sodelovanje → Ponudi agentu (MCP) omogoči agentu UI ali drugemu orodju, da prek protokola Model Context Protocol upravlja odprti urejevalnik. Okno prikazuje lokalni naslov z gumbom **Kopiraj**, **Ustavi strežnik** pa ga konča; dokler teče, naslovna vrstica prikazuje **MCP je aktiven**. Možnost ukazne vrstice `--mcp` zažene isti vmesnik brez okna. Odjemalci lahko odkrijejo orodja za geometrijo, sredstva, lastnosti, animacijo, izvoz in seje. MCP in vtičniki urejevalnika sta različna vmesnika.

# Ukazna vrstica
Program je `heron-brush`. Brez možnosti odpre pozdravni zaslon.

## Odpiranje datotek
`heron-brush map.map [entities.fgd] [materials/]` odpre zemljevid, po želji z definicijami entitet in mapo materialov. Prazen argument enega preskoči, na primer `heron-brush map.map "" materials/`. Če urejevalnik že teče, se zemljevid odpre v njem; `--new-instance` namesto tega odpre ločeno okno.

## Ukazi
- `--help` ali `-h`: izpiše navodila za uporabo. `--version` ali `-V`: izpiše različico.
- `--mcp`: ponuja MCP prek standardnega vhoda in izhoda, brez okna.
- `--merge-map <base> <ours> <theirs> [<output>]`: trismerno združevanje datotek `.map` za git. Izhod je privzeto ours. Spori se izpišejo in vrnejo izhodno kodo, različno od nič.
- `--obj-to-map <in.obj> <out.map> [size] [thickness] [snap] [material]`: pretvori mrežo v brushe. Size je dolžina najdaljše stranice v enotah zemljevida (privzeto 256), snap je mreža za oglišča (0 pomeni brez), besedi `solid` ali `layered` pa namesto tankih klinov zgradita škatle na mreži ali konveksne plošče.
- `--render-preview <in.map> <out.png> [size]`: zapiše sličico zemljevida PNG, privzeto velikosti 512 slikovnih pik.
- `--run-script <script.rhai> <in.map> [out.map] [key=value ...]`: izvede skript vtičnika nad zemljevidom z vsem izbranim in zapiše rezultat (privzeto nazaj v vhodno datoteko). `key=value` odgovarja na nastavitve vtičnika.

# Odpravljanje težav
## Ko je nekaj videti narobe
- Manjkajoč material: preverite mapo ali paket materialov in poti projekta v izvozu.
- Napačna velikost modela: primerjajte z 64 enot visoko oznako igralca in merilom izvoza.
- Napačna smer preliva: preverite kot preliva in usmerjenost ploskve.
- Prazna scena v pogonu: preberite opozorila izvoza in sledite navodilom za uvoz za cilj.
- Povezava v živo čaka: preverite, ali je dodatek nameščen, scena pripeta in posodobitve niso ustavljene.
- Samo črte, brez ploskev: izklopite Žični model v razdelku Pogled na zavihku Zemljevid ali v meniju 3D-okna izberite Materiali ali Polni prikaz.
- Predmeti manjkajo v pogledih: `Ctrl+Shift+I` prikaže skrite predmete; preverite plasti in filtre pogleda.
- Tipka ne naredi ničesar: najprej kliknite v pogled, da dobi tipkovnico, in v Nastavitve → Tipkovnica preverite razpored bližnjic.
- Urejanje ni uspelo: razveljavite, poenostavite geometrijo in uporabite **Preveri zemljevid** in **Preveri puščanja** v razdelku **Težave** na zavihku Zemljevid.
- Zemljevid se iz upravitelja datotek ne odpre: morda je bil predan urejevalniku, ki že teče; poiščite to okno.

## Nastavitve in varnostne kopije
Nastavitve, nedavni zemljevidi, zvezdice, nastavitve izvoza in vtičniki so v mapi s konfiguracijo: `~/.config/heron-brush` v Linuxu in macOS ter `%LOCALAPPDATA%\heron-brush` v sistemu Windows. Varnostne kopije samodejnega shranjevanja so v mapi `autosave` poleg vsakega zemljevida.

## Prijava napake
Navedite različico urejevalnika (Pomoč → O programu), svoj operacijski sistem, majhen zemljevid, ki prikazuje težavo, in natančne korake. Podokno izpisa (`Ctrl+4`) pogosto pove, kaj je šlo narobe. Pred preizkušanjem shranite kopijo.

## Uporaba tega priročnika
Poglavje izberite v kazalu na levi. `Ctrl+F` prikaže kazalo in se premakne v iskalno polje; `Esc` počisti iskanje ali, če je iskanje prazno, zapre priročnik. **Vsebina** skrije ali prikaže seznam poglavij, **Velikost besedila** pa spremeni velikost pisave; oboje se zapomni. Seznam jezikov na vrhu preklaplja med razpoložljivimi prevodi.
