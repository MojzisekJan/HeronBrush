# Pirmieji žingsniai
Heron Brush yra brašais pagrįstas lygių redaktorius. Lygius kuriate iš iškilųjų brašų, dažote ir tekstūruojate jų sieneles, dedate esybes, animuojate duris ir platformas, o rezultatą eksportuojate į žaidimų variklį. Redaktorius taip pat turi integruotų išteklių, parametrinių formų, tiesioginių ryšių su variklių redaktoriais, bendro redagavimo seansų ir statinių vaizdų atvaizdavimą. Šis vadovas pateikiamas kartu su programa ir veikia be interneto.

macOS sistemoje visur, kur šiame vadove parašyta `Ctrl`, veikia `Cmd`, o Option atitinka Alt.

## Pasveikinimo ekranas
Kai Heron Brush paleidžiamas be žemėlapio, atsiveria pasveikinimo ekranas. Kairėje yra **Tęsti** (paskutinis atvertas žemėlapis), **Naujas žemėlapis** ir **Naršyti** esamam `.map` failui atverti, taip pat **Žaidimas**, **Vadovas** ir **Nuostatos**. Dešinėje rodomas paskutinių žemėlapių sąrašas su miniatiūromis, o po juo – **Pavyzdžiai**. Žvaigždutė (**Laikyti viršuje**) prisega žemėlapį sąrašo viršuje; šiukšliadėžė (**Perkelti į šiukšlinę**) perkelia failą į sistemos šiukšlinę, todėl jį galima atkurti. Rodyklių klavišais judama po ekraną, o Enter atveria pasirinktą elementą. Meniu Failas → Sveiki ekraną bet kada grąžina.

## Pirmasis žemėlapis
1. Pasveikinimo ekrane pasirinkite **Naujas žemėlapis** arba Failas → Naujas žemėlapis (`Ctrl+N`).
2. Įrankių juostoje pasirinkite brašo įrankį (`B`) ir vilkite 2D vaizde, kad nupieštumėte pasirinktą formą. Jos gylį patikrinkite kitame 2D vaizde.
3. Pažymėkite brašą ir perkelkite jį vilkdami. Kortele **Sienelė** suteikite jam medžiagą arba spalvą.
4. Išsaugokite šaltinio žemėlapį su `Ctrl+S`. Kai būsite pasiruošę jį naudoti žaidime, atverkite Failas → Eksportuoti ir įrašykite sceną savo varikliui.

## Failai ir išsaugojimas
`.map` failas yra jūsų redaguojamas šaltinis. Išsaugokite reguliariai: eksportavimas, tiesioginiai ryšiai ir atvaizdavimas niekada neišsaugo žemėlapio už jus. Failas → Išsaugoti kaip (`Ctrl+Shift+S`) prideda `.map` plėtinį, jei jo neįrašėte. Failas → Grąžinti išsaugotą žemėlapį atmeta neišsaugotus pakeitimus ir iš naujo įkelia failą. Jei uždarote langą arba atveriate kitą žemėlapį turėdami neišsaugotų pakeitimų, Heron Brush paklausia, ar **Išsaugoti**, **Atmesti**, ar **Atšaukti**.

Kol išsaugotas žemėlapis turi neišsaugotų pakeitimų, kas dešimt minučių į `autosave` aplanką šalia žemėlapio įrašomos sunumeruotos atsarginės kopijos. Saugomos 50 naujausių kopijų.

Jei žemėlapį atveriate, kai Heron Brush jau veikia, jis perduodamas veikiančiam langui, kuris iškeliamas į priekį. Jei norite atskiro lango, komandų eilutėje naudokite `--new-instance` (žr. skyrių „Komandų eilutė“).

# Sąsaja
Pagrindiniame lange yra antraštės juosta su meniu, po ja įrankių juosta, žemėlapio vaizdai, inspektorius dešinėje ir būsenos juosta apačioje.

## Antraštės juosta ir meniu
Antraštės juostoje rodomas logotipas, meniu, žemėlapio pavadinimas viduryje ir lango mygtukai. macOS sistemoje įprasti raudonas, geltonas ir žalias mygtukai yra kairėje. Vilkite antraštės juostą, kad perkeltumėte langą, ir dukart spustelėkite ją, kad jį išdidintumėte. Kai kas nors vyksta fone, juostos gale atsiranda indikatorius: tiesioginis ryšys su varikliu, tiesioginis seansas arba **MCP aktyvus**. Spustelėkite indikatorių, kad sustabdytumėte tiesioginį ryšį arba atvertumėte jo langą.

- **Failas**: Sveiki, Naujas žemėlapis (`Ctrl+N`), Atverti žemėlapį (`Ctrl+O`), Išsaugoti (`Ctrl+S`), Išsaugoti kaip (`Ctrl+Shift+S`), Grąžinti išsaugotą žemėlapį, Eksportuoti, Atvaizduoti vaizdą, Papildiniai ir Nuostatos (`Ctrl+,`).
- **Taisa**: Atšaukti (`Ctrl+Z`), Grąžinti (`Ctrl+Shift+Z`), Iškirpti, Kopijuoti, Įklijuoti, Dubliuoti (`Ctrl+D`), Ištrinti, Pažymėti viską (`Ctrl+A`), Kartoti paskutines komandas (`Ctrl+R`) ir Pamiršti pakartojamas komandas.
- **Rodinys**: 3D vaizdas atskirame lange, Eiti į, Pritraukti prie tinklelio (`Alt+0`), Rodyti tinklelį (`0`), Animacijų peržiūra ir Rodyti praleistas sieneles.
- **Geometrija**: Įterpti 3D tekstą, Sujungti kampą, Paversti sieneles lopais, Iškelti pažymėtus objektus iš jų esybės ir Sinchronizuoti susietas grupes.
- **Animacija**: Raktinių kadrų animacija, Animacijų peržiūra ir Padėti kamerą pagal 3D vaizdą (`Ctrl+Alt+0`).
- **Žemėlapis**: Įkelti, Iš naujo įkelti ir Iškelti taškų failą; Įkelti, Iš naujo įkelti ir Iškelti portalų failą. Jie rodo žemėlapių kompiliatorių įrašytus nuotėkių pėdsakus ir portalus.
- **Bendradarbiavimas**: Tiesioginis seansas ir Teikti agentui (MCP).
- **Pagalba**: Vadovas ir Apie.

## Įrankių juosta
Įrankių juostoje yra visi įrankiai. Pirmoji grupė naudojama nuolat: Pažymėti, Brašas, Pjūvis, Viršūnė, Briauna, Sienelė, Sukti ir Mastelis. Antroje grupėje yra Kreipti, Surinkimas, Protraukimas, Valdymo taškas, Suapvalinimas ir Dažymas. Užveskite žymeklį ant mygtuko, kad pamatytumėte jo pavadinimą ir spartųjį klavišą. Dar kartą spustelėjus aktyvų įrankį grįžtama prie įrankio Pažymėti.

Po įrankių eina **Apversti horizontaliai** ir **Apversti vertikaliai**, tada du jungikliai: **Užrakinti** išlaiko medžiagos išdėstymą, kai brašas juda, o **UV** – kai brašas sukasi (`U` jį perjungia). Toliau rodomi aktyvaus įrankio nustatymai: brašo įrankis rodo formą ir jos parinktis, o suapvalinimo įrankis – spindulį, sieneles ir kampus su mygtuku **Taikyti**. Dešiniajame gale yra įrankio pavadinimas, tinklelio dydis (**Tinklelis 16** ir pan.; spustelėkite, kad pasirinktumėte kitą) ir meniu **Tinklelio nustatymai** su punktais Pritraukti prie tinklelio, Rodyti tinklelį, Animacijų peržiūra, Rodyti praleistas sieneles ir Raktinių kadrų animacija.

## Žemėlapio vaizdai
Yra keturi vaizdai: 3D vaizdas ir trys plokšti vaizdai – Viršus (XY), Priekis (XZ) ir Šonas (YZ). Nuostatos → Rodinys → Išdėstymas leidžia pasirinkti Vienas langas, Du langai, Trys langai arba Keturi langai. Vilkite skirtukus, kad pakeistumėte langų dydį. Spustelėjus vaizdą, jis gauna klaviatūrą. `Space` perjungia langą, rodantį po vieną vaizdą, į kitą vaizdą, o `Ctrl+Space` išdidina klaviatūrą turintį vaizdą ir vėl atkuria išdėstymą.

Spustelėkite dešiniuoju mygtuku nevilkdami bet kuriame vaizde, kad atvertumėte kontekstinį meniu pažymėtiems objektams ir tam, kas yra po žymekliu. Jame galima grupuoti, tvarkyti sluoksnius ir susietas grupes, kurti esybes, perkelti brašus į esybę ar iš jos, kopijuoti medžiagos pavadinimą ir parodyti ją medžiagų naršyklėje.

## Inspektorius
Inspektorius dešinėje turi keturias korteles. Vilkite jo kairįjį kraštą, kad pakeistumėte plotį.

- **Žemėlapis** (`Ctrl+1`): Sluoksniai, Grupės, Papildiniai, Problemos, View filters ir Žymos.
- **Esybė** (`Ctrl+2`): esybių naršyklė, esybių apibrėžimai, pažymėtos esybės savybės, o mygtukams ir paleidikliams – Animacijos paleidiklis.
- **Sienelė** (`Ctrl+3`): UV, Medžiagos, Spalva, Protraukimas, Suapvalinimas, Lopas bei Įtraukimas ir nuožulna. Kiekvieną skyrių galima suskleisti.
- **Ištekliai**: pakartotinai naudojamų brašų modelių biblioteka.

`Ctrl+5` paslepia arba parodo inspektorių, o `Ctrl+Alt+T` paslepia arba parodo įrankių juostą, todėl vaizdams lieka daugiau vietos.

## Būsenos juosta ir išvesties sritis
Būsenos juosta apačioje rodo naujausią redaktoriaus pranešimą. Klaidos rodomos raudonai. Spustelėkite juostą arba paspauskite `Ctrl+4`, kad atvertumėte išvesties sritį su visa pranešimų istorija. Dauguma komandų čia praneša, ką padarė, pavyzdžiui, kiek brašų buvo nupjauta.

## Dialogai ir išvaizda
Dialogai, tokie kaip Nuostatos, Eksportuoti ir Atvaizduoti vaizdą, atsiveria atskiruose languose. Nuostatos → Rodinys → Spalvų schema leidžia pasirinkti Kaip darbalaukyje, Šviesi arba Tamsi. **Skydelių nepermatomumas (%)** puslapyje Apžvalga padaro skydelius virš vaizdų permatomus. Langas pats piešia savo mygtukus pagal jūsų sistemos stilių: macOS – savaiminius, Windows – Windows 11 stiliaus, Linux – GNOME stiliaus.

# Naršymas ir klaviatūra
## Judėjimas 3D vaizde
Laikykite dešinįjį pelės mygtuką ir vilkite, kad apsidairytumėte. Vilkite viduriniu mygtuku, kad paslinktumėte vaizdą. Ratukas juda pirmyn ir atgal. `W`, `A`, `S` ir `D` skrenda pirmyn, kairėn, atgal ir dešinėn, `Q` kyla, o `X` leidžiasi; laikykite `Shift` didesniems žingsniams. Įjungus **Alt + tempimas viduriniu mygtuku judina kamerą** puslapyje Nuostatos → Pelė, Alt su viduriniu mygtuku juda žiūrėjimo kryptimi. Greičiai, apvertimai ir matymo laukas nustatomi puslapyje Nuostatos → Pelė.

## Judėjimas 2D vaizduose
Vilkite viduriniu mygtuku, kad paslinktumėte, ir ratuku keiskite mastelį aplink žymeklį. Slenkant ar keičiant vieno plokščio vaizdo mastelį, kiti lieka sulygiuoti; tai valdo **Susieti 2D vaizdus** puslapyje Nuostatos → Pelė. `Ctrl+Alt+Z` atstato mastelį, o `Ctrl+Shift+U` grąžina plokščius vaizdus į koordinačių pradžią.

`Ctrl+U` įrėmina pažymėtus objektus kiekviename vaizde. Rodinys → Eiti į (`Ctrl+Alt+M`) perkelia kamerą į įvestas koordinates, pažymi tai, kas yra tam tikroje žemėlapio failo eilutėje, arba perkelia pažymėtus objektus įvestu atstumu.

## Tinklelis ir pritraukimas
`+` ir `-` keičia tinklelio dydį, o skaičių klavišai nuo `1` iki `9` tiesiogiai parenka dydį nuo 1 iki 256 vienetų. `0` arba `#` parodo ar paslepia tinklelį, o `Alt+0` įjungia arba išjungia pritraukimą. `S` pritraukia pažymėtų brašų viršūnes prie dabartinio tinklelio, `Ctrl+Shift+V` – prie sveikųjų vienetų, o `Ctrl+Alt+Shift+V` – prie tinklelio.

## Dažniausi spartieji klavišai
- `F3` arba `Ctrl+Shift+P`: ieško bet kurios meniu komandos pagal pavadinimą, o `Enter` ją vykdo. `F10` atveria meniu, o rodyklės juose juda.
- `Ctrl+S`: išsaugoti žemėlapį. `Ctrl+Shift+S`: išsaugoti kaip.
- `Ctrl+Z`: atšaukti. `Ctrl+Shift+Z` arba `Ctrl+Y`: grąžinti.
- `Ctrl+C`, `Ctrl+X`, `Ctrl+V`: kopijuoti, iškirpti ir įklijuoti. Įklijuojant objektai padedami ties žymekliu; `Ctrl+Alt+V` išlaiko jų pradines koordinates. Pažymėjus sieneles, kopijuojama jų medžiaga ir UV nustatymai.
- `Ctrl+D`: dubliuoti. `Ctrl+Shift+D`: susietas grupės dublikatas.
- `Delete` arba `Backspace`: ištrinti pažymėtus objektus.
- `Esc`: atšaukia dabartinį vilkimą, grįžta žingsniu atgal aktyviame įrankyje, grįžta prie įrankio Pažymėti ir galiausiai panaikina žymėjimą. `Shift+Esc` iškart grįžta prie įrankio Pažymėti.
- `Ctrl+R`: pakartoti paskutines pakartojamas komandas. `Ctrl+Shift+R`: jas pamiršti.
- `Ctrl+Alt+0` arba `Ctrl+Alt+Num0`: padėti atvaizdavimo kamerą ties 3D vaizdu.
- `F5`: iš naujo įkelti medžiagas. `F6`: iš naujo įkelti esybių apibrėžimus.

## Žymėjimo spartieji klavišai
- `Ctrl+A`: pažymėti viską arba, kai aktyvus viršūnių, briaunų ar sienelių įrankis, visas rankenėles. `Ctrl+Shift+A`: nieko nežymėti. `Ctrl+Alt+A`: invertuoti žymėjimą.
- `Ctrl+B`: pažymėti kitus tos pačios esybės ar grupės objektus.
- `Ctrl+T`: pažymėti viską, ką liečia pažymėti brašai. `Ctrl+E`: pažymėti viską jų viduje. `Ctrl+Shift+E`: pažymėti viską jų stulpelyje išilgai vaizdo ašies. Žymintys brašai po to sunaudojami.
- `Ctrl+I`: izoliuoti pažymėtus objektus. `Ctrl+Alt+I`: juos paslėpti. `Ctrl+Shift+I`: vėl rodyti viską.
- `Ctrl+G`: grupuoti. `Ctrl+Shift+G`: išgrupuoti.

## Transformavimo spartieji klavišai
- Rodyklių klavišai ir `Page Up` / `Page Down` perkelia pažymėtus objektus vienu tinklelio žingsniu ta kryptimi, kuria jie rodo aktyviame vaizde. Su `Ctrl` jie judant dubliuoja, o su `Alt` – suka.
- `Alt+X`, `Alt+Y`, `Alt+Z`: pasukti ketvirtį apsisukimo apie tą ašį; pridėkite `Shift` priešingai krypčiai.
- `Ctrl+F`: apversti skersai vaizdo; `Ctrl+Alt+F`: apversti aukštyn kojomis. `Ctrl+Shift+X` ir `Ctrl+Shift+Y` atspindi išilgai X arba Y.
- `Insert`: padėti taškinę esybę pažymėtų objektų centre.
- `Tab`: pažymėjus brašą, ciklu perjungti Pažymėti → Viršūnė → Briauna → Sienelė → Pažymėti.

## Blender klavišų schema
Nuostatos → Klaviatūra → Klavišų schema siūlo Heron Brush (numatytoji, naudojama visame šiame vadove) ir Blender. Su Blender: `G` pažymi ir perkelia, `R` suka, `S` keičia mastelį, `E` protraukia, `1`, `2` ir `3` parenka viršūnių, briaunų ir sienelių įrankius, `K` pjauna, `B` žymi, `A` pažymi viską, o `Alt+A` panaikina žymėjimą, `X` ištrina, `Shift+D` dubliuoja, `H` paslepia, o `Alt+H` parodo viską, ir `Ctrl+Shift+Z` grąžina. Visa kita abiejose schemose vienoda.

# Brašai ir modeliavimo įrankiai
Brašai yra iškilieji kūnai, apibrėžti plokštumomis. Sudėtingus objektus kurkite iš kelių paprastų brašų, o ne bandykite padaryti vieną brašą įgaubtą. Kur pakanka paprastos formos, laikykite brašų skaičių mažą.

## Brašų piešimas
Pasirinkite brašo įrankį (`B`) ir vilkite vaizde. Įrankių juostoje rodoma **Piešiama forma**: Stačiakampis gretasienis, Laiptai, Arka, Cilindras, Kūgis, Sferoidas (UV), Sferoidas (ikosaedras) ir Įterpti 3D tekstą. Nuo `Alt+1` iki `Alt+7` formą parenka tiesiogiai. Priklausomai nuo formos, juostoje rodoma **Tuščiaviduris** ir **Storis:**, laiptams – **Pakopos aukštis:** ir **Kryptis:**, apvalioms formoms – **Ašis:**, **Kraštinės:** ir apskritimo režimas, sferoms – **Žiedai:**, o ikosaedrui – **Tikslumas:**. Parinktis nustatykite prieš vilkdami. Rezultatą patikrinkite daugiau nei viename vaizde.

## Žymėjimas
Spustelėkite brašą, kad jį pažymėtumėte, ir spustelėkite su `Ctrl`, kad pridėtumėte prie žymėjimo. Vilkimas nuo tuščios vietos piešia žymėjimo stačiakampį. 3D vaizde spustelėjimas su `Shift` pažymi vieną sienelę, o su `Ctrl+Shift` prideda daugiau sienelių. Vilkdami pažymėtą brašą jį perkeliate; vilkdami nepažymėtą, pažymite ir perkeliate tik jį.

3D vaizde vilkite pažymėto brašo sienelę su `Shift`, kad ją ištemptumėte. Spustelėkite kitą sienelę su `Alt`, kad nukopijuotumėte į ją pažymėtos sienelės medžiagą ir lygiavimą; `Alt+Ctrl` kopijuoja tik medžiagą, o `Alt+Shift` – pasukimą.

## Formavimo įrankiai
- **Pažymėti** (`Esc`): žymėti ir perkelti.
- **Viršūnė** (`V`), **Briauna** (`E`), **Sienelė** (`F`): vilkti kampus, briaunas arba sieneles. Vilkimas su `Shift` nuo briaunos ar sienelės vidurio ten prideda kampą. `Delete` pašalina pasirinktus kampus.
- **Sukti** (`R`), **Mastelis** (`T`), **Kreipti** (`G`): transformuoti pažymėtus objektus rankenėlėmis.
- **Pjūvis** (`C`): spustelėkite du ar tris taškus, kad apibrėžtumėte pjovimo plokštumą. `Enter` pjauna, `Ctrl+Enter` parenka, kurią pusę palikti, o `Backspace` arba `Esc` pašalina paskutinį tašką.
- **Surinkimas** (`N`): išdėstykite taškus aplink tūrį ir paspauskite `Enter`, kad iš jų būtų sukurtas iškilasis brašas. `Backspace` pašalina paskutinį tašką.
- **Protraukimas** (`Y`): žr. skyrių apie protraukimą žemiau.
- **Valdymo taškas** (`P`): keisti lopo valdymo tinklelio formą.
- **Suapvalinimas** (`O`): spustelėkite briauną, kad ją suapvalintumėte, arba pasirinkite briaunas ar viršūnes jų įrankiais ir paspauskite `Enter`. Daugiau sienelių duoda glotnesnį rezultatą ir daugiau geometrijos.
- **Dažymas** (`K`): žr. skyrių „Medžiagos ir UV“.

## Brašų jungimas
- `Ctrl+J`: sulieti pažymėtus brašus į jų iškiląjį apvalkalą.
- `Ctrl+K`: atimti pažymėtus brašus iš brašų, kuriuos jie liečia.
- `Ctrl+Shift+K`: padaryti pažymėtus objektus tuščiavidurius, su vieno tinklelio žingsnio storio sienomis.
- `Ctrl+L`: pažymėtų objektų sankirta.

## Protraukimas, suapvalinimas, lopai ir nuožulna
**Protraukimas** kuria geometriją išilgai kelio iš pažymėtų sienelių. Pažymėkite sieneles (protraukimo įrankyje – spustelėjimu su `Alt`), tada kortelės Sienelė skyriuje Protraukimas nustatykite **Eiga**, **Pasukimas** (ašis ir kampas), **Mastelis**, **Pakopos** ir **Kelias** (lygiavimas Sveikasis skaičius arba Laisvas; Lankas, Tiesi arba S formos vingis) ir paspauskite **Protraukimas**. Eilutė po valdikliais nurodo, kiek brašų bus sukurta.

Skyrius **Suapvalinimas** suapvalina pasirinktas briaunas pagal **Spindulys**, **Sienelės** ir **Kampai** (Su nuopjova arba Suapvalintas); viena sienelė sukuria nuosklembą.

**Lopas** rodo pažymėto lopo **Stulpeliai** ir **Eilutės** su mygtukais porai pridėti ar pašalinti. Geometrija → Paversti sieneles lopais (`Ctrl+P`) iš pažymėtų sienelių sukuria lopus.

**Įtraukimas ir nuožulna** veikia su pažymėtomis sienelėmis: **Įtraukti** įtraukia sienelę nuo jos kraštų, o **Gylis** ją išstumia (arba įstumia, kai neigiamas). **Nuožulninti sieneles** taiko abu; trečiasis mygtukas prideda plokštę arba išpjauna įdubą, priklausomai nuo gylio ženklo. **Sujungti kampą** atveria dialogą, kuris sujungia du brašus nuopjovos arba suapvalintu kampu su peržiūra.

## 3D tekstas
Geometrija → Įterpti 3D tekstą (taip pat paskutinis brašo įrankio formų sąrašo punktas) kuria brašus iš teksto. Naudojamas Heron Block – integruotas daugiakampių šriftas su nupjautais kampais, proporcingais pločiais ir čekiškais diakritikais; mažosios raidės naudoja didžiųjų formas. Įveskite **Tekstas**, nustatykite **Aukštis** (didžiųjų raidžių aukštis; diakritikai iškyla virš jo) ir **Gylis**, stebėkite tiesioginę peržiūrą ir paspauskite **Įterpti**.

Jei operacija negali sukurti tinkamos geometrijos, pakeiskite jos matmenis arba atšaukite ir supaprastinkite formą.

## Masyvai
Geometrija → Masyvas nukopijuoja pasirinkimą ir rodo kopijas žemėlapyje, kol keičiate reikšmes. Kiekiai apima originalą, o masyvas sukuria ne daugiau kaip 1024 kopijas.

- **Eilė** kiekvieną kopiją, palyginti su ankstesne, pastumia per **Žingsnis X**, **Žingsnis Y** ir **Žingsnis Z** ir pasuka per **Pasukimas kopijai (laipsniai)** apie pasirinktą ašį.
- **Tinklelis** užpildo **Kiekis X**, **Kiekis Y** ir **Kiekis Z** elementų **Žingsnis X**, **Žingsnis Y** ir **Žingsnis Z** atstumais.
- **Žiedas** paskirsto elementus per **Kampas (laipsniai)** aplink centrą, nutolusį **Spindulys (vienetai)** nuo pasirinkimo. Su **Atsukti į centrą** kiekviena kopija pasisuka kartu su žiedu.
- **Išbarstymas** padeda kopijas atsitiktinėse vietose spindulio ribose, skersai pasirinktos ašies. Ta pati **Sėkla** duoda tą patį išdėstymą, o **Atsitiktinis pasukimas** pasuka kiekvieną kopiją.

Kai pasirinkta viena visa grupė, **Susietos kopijos** kiekvieną kopiją paverčia susietu dublikatu, todėl vienos pakeitimas pasiekia visas. **Taikyti** palieka kopijas ir pažymi jas kartu su originalu; **Atšaukti** jas pašalina. Visas masyvas atšaukiamas vienu žingsniu.

## Reljefo lipdymas
Reljefą sudaro daug mažų brašų, kurių paviršių lipdymo teptukas stumia ir traukia. Pažymėkite vieną ar kelis brašus ir pasirinkite Geometrija → Padalyti į stulpus, tada stulpų formą: Trikampis, Kvadratas arba Šešiakampis. Brašai supjaustomi išilgai tinklelio linijų į vieno tinklelio žingsnio pločio stulpus, o stulpai sudedami į grupę Terrain. Pjūviai eina pagal pasaulio tinklelį, todėl greta esantys brašai supjaustomi tose pačiose vietose ir turi bendrus kampus. Kiekvienas stulpas stovi išilgai ploniausios brašo pusės: stačias grindyse ar lubose, gulsčias sienoje. Urvas daromas taip pat: išskobkite gretasienį (Hollow selection) su vieno tinklelio žingsnio sienų storiu, pažymėkite sienas ir jas padalykite.

Pasirinkimą galima padalyti ir lange **Lipdymas**, kurį atidaro mygtukas lipdymo juostos gale, į formą, pasirinktą jo eilutėje Stulpų forma. Abi vietos įsimena paskutinę naudotą formą. **Trikampis** sukuria du stulpus kiekvienam tinklelio langeliui, ir trikampio viršus lieka plokščias, kad ir kaip jį lipdytumėte. **Kvadratas** sukuria vieną stulpą langeliui, tai yra perpus mažiau brašų; jo viršus sulinksta išilgai įstrižainės. **Šešiakampis** padengia brush'ą dviejų tinklelio žingsnių pločio šešiakampiais, kurių kiekvienas padalijamas į šešis trikampius aplink centrą, todėl kiekvienas viršus lieka plokščias.

Pasirinkite įrankį **Lipdymas** (`J`) ir vilkite per paviršių 3D vaizde. Jei laikote mygtuką nejudindami pelės, teptukas toliau dirba toje pačioje vietoje, dešimt kartų per sekundę, todėl kalva auga tol, kol laikote. Oranžinis žiedas rodo teptuką. Kai niekas nepažymėta, teptukas keičia visus po juo esančius brašus, o kai yra pažymėtų, tik pažymėtus. Juosta lango viršuje nustato, ką daro brūkšnys:

- **Pakelti** ir **Nuleisti** stumia paviršių į išorę arba į vidų, labiausiai žiedo centre, o krašte visai ne. Laikykite `Shift`, kad juos sukeistumėte.
- **Išlyginti nelygumus** nugludina kauburius.
- **Išlyginti** išlygina paviršių iki vietos, kurioje brūkšnys prasidėjo.

Tame pačiame lange **Kryptis** nustato, kuria kryptimi:

- **Aukštyn** stumia paviršių tiesiai aukštyn ir žemyn, apskritime ant žemės. Tai skirta reljefui.
- **Kamera** stumia jį artyn prie vaizdo ir tolyn nuo jo, rutulyje aplink žymeklį. Pažiūrėkite į grindis, sieną ar lubas, ir lipdomas bus tai, į ką žiūrite.
- **Normalė** stumia jį į išorę iš sienelės, kuri yra po žymekliu ten, kur brūkšnys prasideda.

Du skaičiai yra **Spindulys (vienetai)** ir **Stiprumas (%)**. Juda tik kampai toje pusėje, kurią brūkšnys stumia; galinė pusė lieka vietoje, o stulpas lieka bent vieno vieneto storio. Bendrą kampą brašai pastumia kartu, todėl paviršius lieka uždaras. Brūkšnys atšaukiamas vienu žingsniu. 2D vaizduose įrankis žymi kaip žymėjimo įrankis.

Baigę lipdyti, pažymėkite reljefą ir pasirinkite Geometrija → **Sujungti stulpus** arba mygtuką lange **Lipdymas**. Gretimi stulpai, kurie kartu sudaro iškilą brush'ą ir atrodo vienodai, tampa vienu brush'u, todėl lygūs plotai virsta keliais dideliais brush'ais. Paviršius nepajuda, o sujungimą galima atšaukti vienu žingsniu.

**Sujungti išlaikant formą**, esantis greta meniu ir lange, jungia taip pat, bet nepaiso sienelių išvaizdos: kur dvi sienelės tampa viena, ji gauna vienos iš jų medžiagą. Veikia su bet kokiais pažymėtais brush'ais, ne tik su reljefu. Keturios dėžės L raide tampa dviem, niekada viena, kuri užpildytų L, o visas nulipdytas reljefas sumažėja iki tiek brush'ų, kiek pavyksta rasti jungiant kaimynus. Jungiami tik tos pačios grupės ar esybės brush'ai, todėl durys niekada nepasiglemžia šalia esančios sienos.

Reljefui kaip vokselinių žaidimų padalykite į **Kvadratas** stulpus ir lange **Lipdymas** prie **Brūkšnys** pasirinkite **Blokai**. Brūkšnys tada kelia visą kiekvieno stulpo viršų visais tinklelio žingsniais ir palieka šonus stačius, todėl reljefas kyla laiptais. Visi režimai veikia kaip anksčiau; maži brūkšniai sumuojasi, kol stulpas pakyla visu žingsniu. Kitų formų stulpai lieka nepakitę.

**Laisvai** nereikia stulpų – veikia su bet kokiais brush'ais. Pakelti prideda žemės rutulyje aplink žymeklį, o Nuleisti jos atima, todėl reljefas gali turėti iškyšas ir urvus. Išlyginti nelygumus jį suvienodina, o Išlyginti nuleidžia iki lygio, kur prasidėjo brūkšnys. Kiekvienas prisilietimas tinklelyje iš naujo sudaro paliestus brush'us su suapvalinimu, o viską už rutulio palieka kaip buvo, net aštrias briaunas. Rezultatas – įprasti brush'ai, todėl jiems tinka visi kiti įrankiai, o **Sujungti stulpus** paskui sumažina jų skaičių. Smulkesnis tinklelis duoda lygesnį paviršių ir daugiau brush'ų.

# Medžiagos ir UV
## Medžiagų įkėlimas ir taikymas
Atverkite kortelės Sienelė skyrių **Medžiagos**. **Įkelti aplanką** prideda paveikslėlių aplanką, o **Įkelti .wad** – tekstūrų paketą; abu atsiranda po **Kolekcijos**, kur kryželis vieną pašalina. Rašykite į **Filtruoti medžiagas**, kad susiaurintumėte sąrašą. Spustelėkite medžiagą, kad ji taptų dabartine, tada **Taikyti pažymėtiems** uždeda ją ant kiekvienos pažymėtos sienelės. **Pažymėti** siūlo Pažymėti brašus su ja ir Pažymėti sieneles su ja. **Pakeisti** sukeičia vieną medžiagą kita visame žemėlapyje arba tik pažymėtuose objektuose.

Įrankis **Dažymas** (`K`) uždeda dabartinę medžiagą ant sienelės po žymekliu: spustelėjimas nudažo vieną sienelę, dvigubas spustelėjimas – visą brašą, o spustelėjimas su `Shift` paima sienelės medžiagą kaip dabartinę, tarsi pipete.

## UV redaktorius
Kortelės Sienelė skyrius **UV** rodo pažymėtos sienelės medžiagą ir kontūrą. Pelės valdymas drobėje:
- Ratukas keičia mastelį aplink žymeklį. Vilkite dešiniuoju arba viduriniu mygtuku, kad paslinktumėte. Dukart spustelėkite dešiniuoju mygtuku, kad vėl įrėmintumėte sienelę; pažymėjus kitą sienelę, ji įrėminama automatiškai.
- Vilkimas kairiuoju mygtuku perkelia medžiagą, o sienelės kontūras lieka vietoje. Žiedas suka išdėstymą, kampų rankenėlės keičia jo mastelį, o centrinė rankenėlė nustato šių pakeitimų pradžios tašką.
- Vilkimas kairiuoju mygtuku su `Alt` ant plytelės krašto kreipia, kaip ir vilkimas viduriniu mygtuku su `Shift`.
- Kiekvienas vilkimas yra vienas atšaukimo žingsnis, o `Esc` atšaukia vykstantį vilkimą.

Po drobe yra **Poslinkis**, **Mastelis**, **Kampas** ir, jei žaidimas juos naudoja, **Turinio vėliavėlės**, **Paviršiaus vėliavėlės** ir **Paviršiaus reikšmė**. Mygtukai yra **Atstatyti**, **Atstatyti pagal pasaulį**, **Pritaikyti**, **Apversti H**, **Apversti V** ir du 90° pasukimai. **Lygiuoti** pasuka medžiagą pagal vieną iš sienelės briaunų (paspauskite dar kartą kitai briaunai, su `Shift` – ankstesnei). Keturi lygiavimo mygtukai prispaudžia medžiagą prie krašto. **Pritaikyti U** ir **Pritaikyti V** keičia mastelį taip, kad sienelę dengtų sveikas kopijų skaičius; paspauskite dar kartą kitam skaičiui ir laikykite `Ctrl`, kad būtų išlaikytas apdailos lapo (trim sheet) padalijimas.

## UV klavišai
Pažymėjus sieneles, rodyklių klavišai perkelia medžiagą, `[` ir `]` pasuka ją 15°, `,` ir `.` perpus sumažina ar padvigubina jos mastelį, `Ctrl+F` ir `Ctrl+Alt+F` ją apverčia, `Shift+R` ją atstato, `Alt+Shift+R` atstato pagal pasaulio ašis, o `Ctrl+Shift+F` pritaiko ją kiekvienai sienelei.

**Užrakinti** įrankių juostoje išlaiko išdėstymą, kol brašai juda, o **UV** (`U`) – kol brašai sukasi.

## Įrankių medžiagos
- skip: pašalina pažymėtą sienelę iš eksporto į variklius.
- clip: sukuria susidūrimus be matomos geometrijos.
- origin: pažymi brašo esybės sukimo centrą.

Pavadinimus galima pakeisti eksporto lango kortelėje **Įrankių medžiagos**. Skip naudokite sienelėms, apie kurias žinote, kad jos paslėptos.

## Sienelių praleidimas eksportuojant
Pažymėkite sienelę ir skyriuje UV įjunkite **Neįtraukti sienelės į eksportą**. UV peržiūroje ji pažymima oranžiniu brūkšniavimu. Jos medžiaga ir dažai lieka išsaugoti, o brašas lieka uždaras ir redaguojamas. Eksportuoti atvaizdavimo tinklai šios sienelės neturi, bet susidūrimai išlieka. Paliekant vieną kubo keturkampį, susidaro du atvaizduojami trikampiai. Tai nepadaro sienelių dvipusių ir nesujungia tinklų.

**Praleisti uždengtas sieneles** pažymi sieneles, kurias visiškai dengia statiniai brašai, įskaitant identiškas besiliečiančias sieneles ir uždarus brašus. Veikia su pažymėtais brašais arba, jei niekas nepažymėta, su visa matoma neužrakinta pasaulio geometrija. Brašų esybės, permatomos medžiagos ir brašai, kurie jau turi praleistų sienelių, praleidžiami. Iš dalies uždengtos sienelės neskaidomos. Vienas atšaukimas viską atkuria. Rodinys → Rodyti praleistas sieneles paslepia arba parodo tokias sieneles redaktoriaus vaizduose; žymint vis tiek naudojamas visas brašas.

# Spalva ir gradientai
Kortelės Sienelė skyrius **Spalva** dažo sieneles vienspalvėmis spalvomis arba gradientais ant jų medžiagų. Tai naudinga lygio maketavimui ir stilizuotai grafikai. Nudažytos sienelės vaizduose išlaiko savo tikras spalvas; žymėjimas rodomas kontūru.

## Paletė ir maišymo spalva
Spustelėkite vieną iš dvylikos paletės pavyzdžių, kad juo nudažytumėte pažymėtas sieneles arba visus pažymėtus brašus. **Nepermatomumas** nustato, kiek nepermatomi bus kiti dažai, procentais. **Maišyti** turi spalvos mygtuką, kuris atveria spalvų parinkiklį; vilkite parinkiklyje ir pažymėti objektai keičiasi tiesiogiai, arba paspauskite **Dažymas**, kad pritaikytumėte maišymo spalvą vienu žingsniu. **Atgal į medžiagą** pašalina dažus, kad sienelė vėl rodytų savo medžiagą. Eilutė skyriaus viršuje nurodo, kas pažymėta ir kuo tai padengta.

## Spalvų parinkiklis
Kiekvienas redaktoriaus spalvos mygtukas atveria tą patį parinkiklį. Vilkite kvadrate, kad pasirinktumėte sodrumą ir šviesumą, o juostoje šalia jo – atspalvį. Žemiau yra **R**, **G** ir **B** reikšmės bei slankiklis **Nepermatomumas**, kai spalva jį turi. Šešioliktainis laukas priima `#RRGGBB` arba `#RRGGBBAA`; jis tampa raudonas, kol tekstas nėra tinkama spalva. **Kopijuoti** įdeda šešioliktainį kodą į iškarpinę. Spustelėkite už parinkiklio ribų, kad jį uždarytumėte.

## Gradientai
**Gradientas** leidžia pasirinkti Tiesinis gradientas arba Radialinis gradientas, o mygtukas šalia jo sukeičia spalvų galus. Kiekvienas taškas turi spalvos mygtuką ir **Padėtis (%)**, o taškai tarp galų turi šalinimo mygtuką. **Pridėti spalvos tašką** prideda tašką plačiausiame tarpe; gradientas gali turėti iki aštuonių taškų. **Gradiento kampas** nustato tiesinio gradiento kryptį, su mygtukais **+45°** ir **−45°**.

Po peržiūra yra bėgelis su žyme kiekvienam taškui. Spustelėkite bėgelį, kad pridėtumėte tašką, spustelėkite žymę, kad atvertumėte jos parinkiklį, vilkite žymę, kad ją perkeltumėte, ir spustelėkite žymę dešiniuoju mygtuku, kad ją pašalintumėte. Trys pavyzdžiai po bėgeliu yra išankstiniai nustatymai: Šiltas gradientas, Šaltas gradientas ir Pilkų atspalvių gradientas. Gradiento pakeitimai pažymėtiems objektams taikomi tiesiogiai.

Gradiento kryptis svarbi: orientuokite jį išilgai stogo šlaito ar išlenkto objekto, o ne skersai nesusijusių sienelių. Nudažę patikrinkite modelį iš abiejų pusių.

## Paviršiaus medžiaga
**Paviršiaus medžiaga** nustato **Šiurkštumas** (0 – blizgus, 100 – matinis), **Metališkumas** (0 medžiui, akmeniui ar plastikui, 100 metalui) ir **Spinduliavimas** (nuo 0 iki 8; paviršius švyti savo spalva, įskaitant gradientus, bet neapšviečia netoliese esančių objektų). Pakeitimai tiesiogiai taikomi pažymėtoms sienelėms ar brašams. **Naudoti numatytąjį paviršių** pašalina perrašymą. Visus pakeitimus galima atšaukti.

Paviršiaus efektai matomi apšviestame 3D vaizde ir kameros atvaizdavimuose. Plokščias ir vielinio karkaso rodymas atšvaitų sąmoningai nerodo. Švytėjimas gali pridėti spindesį aplink spinduliuojančius paviršius. Godot ir glTF/GLB gauna medžiagos reikšmes, o Unity importuotojas šiurkštumą paverčia glotnumu. Kiekvieno tikslo apribojimai aprašyti `docs/surface-materials.md`.

# Ištekliai, sluoksniai ir grupės
## Išteklių biblioteka
Atverkite kortelę **Ištekliai**. Pridėkite `.map` modelių aplanką mygtuku **Pridėti aplanką**; integruota biblioteka yra visada. Rašykite į **Filtruoti išteklius**, kad ieškotumėte. Spustelėkite modelį, kad jį pažymėtumėte, tada dukart spustelėkite jį, paspauskite `Enter` arba spustelėkite **Įterpti**, kad jis būtų pridėtas ten, kur paskutinį kartą buvo žymeklis. Modelį taip pat galite nuvilkti į vaizdą. Išteklių brašai įterpiami kaip viena grupė, todėl modelis juda kaip visuma.

**Išsaugoti pažymėtus objektus kaip išteklių** įrašo pažymėtus brašus į biblioteką kaip `.map`. Bibliotekos ištekliai yra įprasti redaguojami brašai, o ne importuoti tinklai.

## Sluoksniai
Kortelės Žemėlapis sąrašas **Sluoksniai** padeda tvarkyti dideles sritis. Įveskite pavadinimą ir spustelėkite **Pridėti**, kad sukurtumėte sluoksnį, arba paspauskite `Enter`, kad pervadintumėte dabartinį. Kiekvieną sluoksnį galima paslėpti, užrakinti arba neįtraukti į eksportą. **Perkelti čia** perkelia pažymėtus objektus į dabartinį sluoksnį. Kontekstinis meniu vaizduose taip pat siūlo Perkelti į sluoksnį, Padaryti aktyviu sluoksniu, Slėpti sluoksnius ir Izoliuoti sluoksnius.

## Grupės ir susietos grupės
`Ctrl+G` sugrupuoja pažymėtus objektus, o `Ctrl+Shift+G` juos išgrupuoja. Kortelės Žemėlapis skyrius **Grupės** išvardija grupes ir siūlo **Atverti**, **Pervadinti**, **Sulieti**, **Išgrupuoti**, **Išimti** ir **Išskirti**. Susieta grupė yra kopijų rinkinys, kurios dalijasi pakeitimais: `Ctrl+Shift+D` sukuria susietą dublikatą, Geometrija → Sinchronizuoti susietas grupes atnaujina kopijas, o **Atsieti** padaro kopiją nepriklausomą. Kortelėje Esybė savybę galima palikti tik vienos kopijos nuosavybe, kai susietos grupės sinchronizuojamos.

## Bendras mastelis
Numatytasis eksporto mastelis yra 32 žemėlapio vienetai metre. Integruotas žaidėjo pradžios žymeklis yra 64 vienetų aukščio – tai 2 metrų atskaita. Lyginkite su juo durų angas, transporto priemones, baldus ir augalus. Eksporto mastelį keiskite tik sąmoningai ir išlaikykite susijusius išteklius nuoseklius.

# Esybės ir žaidimo eiga
Dažniausi esybių apibrėžimai yra integruoti ir visada prieinami: šviesos, žaidėjo pradžios taškai, durys, mygtukai, platformos, paleidikliai, statiniai rekvizitai, garsai ir maršruto taškai. Įkėlus projekto FGD per **Esybių apibrėžimai** kortelėje Esybė, to paties pavadinimo klasės pakeičiamos, o kitos integruotos lieka.

## Esybių dėjimas
Kortelės Esybė naršyklėje **Esybės** rašykite į **Filtruoti klases** ir pasirinkite klasę. Taškinei klasei, pavyzdžiui, šviesai, spustelėkite **Padėti** arba dukart spustelėkite klasę; `Insert` padeda ją pažymėtų objektų centre. Brašų klasei, pavyzdžiui, durims ar paleidikliui, pažymėkite brašus ir spustelėkite **Sukurti**. Klases taip pat galima nuvilkti į vaizdą.

## Savybės
Savybių sąrašas redaguoja pažymėtą esybę. Tos pačios klasės esybės redaguojamos kartu. **Pridėti savybę** prideda raktą, kurio apibrėžimas gali nežinoti; **Numatytieji** gali grąžinti esamas savybes į numatytąsias reikšmes arba pridėti trūkstamas. Savybės, kurias klasė apibrėžia, bet esybė neturi, rodomos blankiai; įvedus reikšmę jos pridedamos. Esybėms susieti naudokite targetname ir target.

## Ką gali eksportas
Godot direktyvos sukuria savaiminius mazgus, susidūrimų kūnus ir paleidiklių sritis. Unity ir Unreal palaiko savaimines dažniausias šviesas, o Unreal taip pat sukuria PlayerStart aktorius. Kiti duomenys išsaugomi tiek, kiek leidžia tikslo importuotojas.

Vien durų esybė nesuteikia žaidimo elgsenos. Judėjimą, reakcijas į paleidiklius, žaidėjo valdiklius ir projekto skriptus vis tiek reikia sukurti variklyje. Defold eksportuoja esybių savybes ir susidūrimų objektus; Flax išdėsto modelius ir bendrines esybes. Prieš kurdami žaidimo eigą, patikrinkite rezultatą savo variklyje.

# Animacija
Heron Brush gali peržiūrėti paprastą judėjimą ir redaguoti brašų esybių raktinių kadrų animacijas, o tada eksportuoti jas į kelis variklius.

## Animacijų peržiūra
Įjunkite **Animacijų peržiūra** meniu Rodinys, meniu Animacija, meniu Tinklelio nustatymai arba kortelės Žemėlapis skyriuje Rodinys. Ji atkuria func_rotate sukimąsi ir func_shuttle judėjimą pirmyn ir atgal, kai įjungtas start_on, naudodama esybės ašies, greičio, krypties, eigos ir laukimo savybes, taip pat atkuria raktinių kadrų klipus, kuriems nustatyta Paleisti pradedant. Ventiliatoriaus ir vėjo malūno ištekliai jau turi besisukančių dalių.

Išjungus peržiūrą, atkuriama pradinė poza. Spustelėjimas vaizde sustabdo atkūrimą prieš žymint ar redaguojant; kameros naršymas veikia toliau. Peržiūra nekeičia žemėlapio, atšaukimų istorijos ar eksportų ir nevykdo fizikos ar žaidimo logikos.

## Raktinių kadrų redaktorius
Atverkite jį per Animacija → Raktinių kadrų animacija arba iš meniu Tinklelio nustatymai. Jis prisitvirtina po vaizdais arba 3D lange, kol šis yra atskiras. Lango piktograma jo antraštėje perkelia jį į atskirą langą; kadras ir atšaukimų istorija išlieka. **Redaguoti animaciją** perjungia vaizdus į animuotą pozą, o **Redaguoti žemėlapį** grąžina į ramybės geometriją. Animacijos režime kiekvieno vaizdo apačioje rodoma **Redaguoti animaciją** ir kadro numeris.

Pažymėkite brašo esybę arba brašą ar brašų grupę, kad ją animuotumėte. Plokščia brašų grupė dalijasi viena animacija ir sukimo centru; grupėms, kuriose yra kelios esybės, reikia atskirų animacijų.

## Klipai
Parinkiklis **Animacija** perjungia pavadintus klipus. Įveskite pavadinimą į **Animacijos pavadinimas** ir spustelėkite **Išsaugoti kaip**, kad nukopijuotumėte dabartinį klipą tuo pavadinimu; **Ištrinti** pašalina klipą. **Perstumti** sukuria `translate` klipą be sukimo. **Durų animacijos** sukuria sklandžius `open` ir `close` klipus su 90 laipsnių pasukimu apie dabartinį sukimo centrą; pirmiausia perkelkite sukimo centrą prie vyrio. **Paleisti pradedant** paleidžia klipą, kai lygis prasideda; durų šablonas tai išjungia.

## Valdymas ir atkūrimas
Valdymo mygtukai yra **Pradžia**, **Ankstesnis raktinis kadras**, **Atkūrimas**, **Kitas raktinis kadras** ir **Pabaiga**. **Kadras** rodo ir nustato dabartinį kadrą, o **Trukmė (s)** nustato klipo ilgį. Režimas yra **Vieną kartą**, **Kartojimas** arba **Pirmyn ir atgal**.

## Laiko juosta
Laiko juostoje yra liniuotė su atkūrimo žymekliu ir takeliai – **Padėtis** (X, Y ir Z kaip vienas raktinis kadras) ir kiekvienam sukimo kampui RX, RY ir RZ. Spustelėkite arba vilkite liniuotėje, kad perkeltumėte atkūrimo žymeklį ir peržiūrėtumėte tą pozą visuose vaizduose. Vilkite viduriniu mygtuku, kad paslinktumėte laiko juostą. Vilkite raktinius kadrus, kad perkeltumėte juos laike; `Shift` prideda prie žymėjimo; dukart spustelėkite takelį, kad įterptumėte raktinį kadrą; `Ctrl`+ratukas keičia mastelį. **Kreivės** vietoj takelių rodo reikšmių kreives.

Mygtukai virš jos yra **Pridėti raktinį kadrą**, **Pašalinti**, **Kopijuoti**, **Įklijuoti** ir **Dubliuoti**. **Pritraukti prie kadrų** laiko raktinius kadrus ant sveikų kadrų, o **Pritaikyti** parodo visą klipą. **Perskirstyti raktinių kadrų laiką** atveria **Laiko mastelis**, **Poslinkis (kadrais)**, **Keisti mastelį nuo atkūrimo žymeklio**, **Pailginti trukmę** ir **Perskirstyti pažymėtų laiką**.

Raktinių kadrų inspektorius šalia laiko juostos redaguoja pažymėtą raktinį kadrą: **Kadras**, **Reikšmė** (arba tris padėties raktinio kadro koordinates), interpoliaciją **Tiesinis**, **Pastovi** arba **Bezjė**, **Įeinantis nuolydis** ir **Išeinantis nuolydis** bei **Glotnios liestinės**.

## Klavišai redaktoriuje
Kai laiko juosta turi klaviatūrą: `Space` paleidžia arba pristabdo, kairė ir dešinė rodyklės pereina po vieną kadrą, `Ctrl+A` pažymi visus raktinius kadrus, `Ctrl+C`, `Ctrl+X` ir `Ctrl+V` kopijuoja, iškerpa ir įklijuoja raktinius kadrus, `Ctrl+D` juos dubliuoja, `Delete` juos pašalina, `Ctrl+Z` atšaukia, o `Esc` atšaukia vilkimą. Už laiko juostos ribų `Esc` išeina iš animacijos režimo. Atšaukimas palieka mastelį, žymėjimą ir atkūrimo žymeklį ten, kur jie buvo.

## Pozavimas vaizduose
Eilutėje po laiko juosta yra **Svogūno sluoksniai** su kiekiu ir **Tarpai**, kurie rodo artimas pozas kaip šešėlius; **Sukimo centras** koordinatės; **Automatiniai raktiniai kadrai**; ir pozavimo įrankiai **Perstumti**, **Sukti** ir **Sukimo centras**.

- **Pridėti raktinį kadrą** įjungia automatinius raktinius kadrus ir pristabdo atkūrimą. Kai jie įjungti ir pasirinkta **Perstumti**, 3D vaizde vilkite spalvotą pažymėtos esybės rankenėlės ašį, kad įrašytumėte jos padėtį dabartiniame kadre.
- 2D vaizde vilkite pažymėtą brašą arba jo grupės narį, kad įrašytumėte perstūmimą išilgai dviejų matomų ašių, pritraukiant prie tinklelio. Tam automatinių raktinių kadrų įjungti nereikia.
- Su **Sukti** vilkite brašą 2D vaizde aplink jo oranžinę sukimo centro žymę. Viršus įrašo RZ, Priekis – RY, o Šonas – RX, pritraukiant prie tinklelio kampų.
- Su **Sukimo centras** vilkite geltoną sukimo centrą bet kuriame 2D vaizde. Jis pritraukiamas prie tinklelio ir išlaiko savo gylį. Raktiniai kadrai ir ramybės geometrija nesikeičia.

Žydras kelias 2D ir 3D vaizduose rodo sukimo centro judėjimą. Spustelėkite kelio tašką, kad pasirinktumėte jo laiką ir raktinius kadrus, arba vilkite jį, kad pakeistumėte padėtį; įjungus automatinius raktinius kadrus, kelio taškus galima vilkti ir 3D vaizde. Etiketės rodo kadrų numerius, o brūkšninės atkarpos žymi šuolius. `Esc` atšaukia bet kurį iš šių vilkimų, o vienas atšaukimas grąžina visą vilkimą.

Naujos animacijos prasideda ramybės būsenoje su raktiniais kadrais laiko nulyje. Judesį sukurkite pakeisdami pozą kitame kadre. Aštriam L formos keliui įrašykite pradžią, kampą ir pabaigą ir naudokite Tiesinis su Pirmyn ir atgal.

## Mygtukų susiejimas su animacijomis
Pažymėkite func_button, trigger_multiple arba trigger_once. Kortelėje Esybė virš savybių rodomas **Animacijos paleidiklis**. Pasirinkite tikslą iš sąrašo arba spustelėkite **Parinkti tikslą vaizde** ir tada animuotą objektą bet kuriame vaizde; `Esc` atšaukia parinkimą. Trūkstami tikslai ar klipai nurodomi po valdikliais. Kai ryšių rodymas įjungtas, esybių ryšiai rodo rodykles į savo tikslus.

**Veiksmas** nustato, ką daro aktyvavimas: **Paleisti** paleidžia pavadintą klipą arba pasirinktą animaciją; **Perjungti** kaitalioja `open` ir `close`, kai jie yra, kitaip iš naujo paleidžia pasirinktą animaciją; **Atverti** ir **Uždaryti** paleidžia tuos klipus; **Pristabdyti** sustabdo pozą, **Tęsti** tęsia, o **Stabdyti** atkuria pradinę pozą. **Tik vieną kartą** perrašo klasės numatytąją reikšmę. **Kartojimo delsa (s)** nustato trumpiausią laiką tarp aktyvavimų; pirmojo ji neatideda.

**Išbandyti paleidiklį** įvykdo veiksmą peržiūroje, neredaguodamas žemėlapio ir nepridėdamas atšaukimo žingsnių. **Atstatyti peržiūrą** atkuria pradines pozas ir išvalo vienkartinio veikimo bei pertraukos būseną. Tai nemodeliuoja žaidėjo susidūrimų ar žaidimo skriptų.

## Animacija eksportuose
Godot eksportuoja savaiminius AnimationPlayer ir AnimationLibrary išteklius, įskaitant kreives ir judančius susidūrimus. CharacterBody3D, patekęs į mygtuko sritį, aktyvuoja jo tikslą. Godot programoje atlikti šių klipų pakeitimai negrįžta į žemėlapį ir pakeičiami kito eksporto metu.

Raylib eksportas prideda `<map>_animation.h` su C grotuvu ir `<map>_preview.c` su laisvos kameros peržiūra; ją sukompiliuokite pagal `<map>_raylib.md`. Peržiūroje `SPACE` pristabdo, `R` iš naujo paleidžia numatytuosius klipus, o `N` perjungia pavadintus klipus. Defold įterpia atkūrimą į esybės skriptą su dokumentuotais `heron_*_animation` pranešimais. Unity Build Level prideda `HeronBrushAnimation` ir kinematinį kūną. Bevy eksportas apima `heron_animation.rs` ir instrukcijas. Godot, Unity, Defold, Bevy ir raylib išlaiko paleidiklių ryšius, vienkartinio veikimo žymą ir kartojimo delsą. Jūsų žaidimas vis tiek turi užregistruoti grotuvą ir prijungti savo mygtukus prie kiekvienos vykdymo aplinkos API.

## Pavyzdys Motion lab
Atverkite `motion_lab` iš pasveikinimo ekrano skyriaus Pavyzdžiai. Jame yra platforma L formos keliu, liftas, besisukanti platforma ir ant vyrių kabančios durys su mygtukais. Atverkite šalia esantį `godot/project.godot` programoje Godot ir paspauskite `F5`. WASD ir pelė judina žaidėją, Space – šuolis, nuo 1 iki 4 nukelia į stoteles, o R iš naujo paleidžia dabartinę stotelę. Užlipkite ant spalvotos plokštės, kad aktyvuotumėte jos tikslą. Pridėtas README paaiškina pakartotinį eksportavimą.

# Eksportavimas į variklį
Failas → Eksportuoti atveria eksporto langą. Kairėje su logotipais išvardyti varikliai: Godot, Defold, three.js, Babylon.js, PlayCanvas, Bevy, raylib, Unity, Unreal Engine, Flax Engine, Cocos Creator, Wavefront .obj, glTF .gltf, GLB .glb ir paskutinis `.map`. Puslapis dešinėje nurodo, ką gaus tikslas. **Įrašyti** jį įrašo, **Tikrinti** sukuria lygį jo neįrašydamas ir išvardija problemas, o **Uždaryti** uždaro langą.

Eksportavimas yra atskiras nuo išsaugojimo: `.map` lieka jūsų redaguojamu šaltiniu. Kiekvienas valdiklis išsaugo savo variklio nustatymus vos tik pasikeičia, todėl kiekvienas variklis išlaiko savo atsakymus, o langas atsiveria toks, kokį jį palikote.

## Bendrosios parinktys
- **Mastelis**: **Vienetų metre**, numatytasis 32.
- **Ką įrašyti**: **Susidūrimų geometrija**.
- **Optimizacijos**: **Grupuoti statinius tinklus 1024 vienetų langeliais**, **Pašalinti geometrijos viduje paslėptas sieneles**, **Sujungti vienoje plokštumoje esančias sieneles** ir **Sujungti susidūrimų formas**.
- **Projekto keliai**: kur projekte patenka lygis ir jo ištekliai.
- **Įrankių medžiagos**: clip, skip ir origin medžiagų pavadinimai.
- **Daugiau**: konkrečiam varikliui skirti jungikliai, tokie kaip **Pavadinti mazgus pagal**, **Atkurti sluoksnius ir grupes kaip tėvinius mazgus**, **Apversti trikampių sukimo kryptį**, tekstūrų koordinačių apvertimas ir antrasis UV rinkinys keptai šviesai su **Šviesos žemėlapio tekselis**.
- **Neatsiliekant nuo žemėlapio**: **Įrašyti iš naujo kaskart išsaugojus žemėlapį**. Ši parinktis tampa prieinama po pirmojo eksporto į variklį, nes tas eksportas nurodo, kur rašyti.

Statiniai langeliai priskiria pasaulio brašus pagal jų centrus ir sujungia juos pagal medžiagą kiekviename 1024 vienetų langelyje, kad varikliai galėtų atmesti mažesnes žemėlapio dalis. Susidūrimai išlaikomi, o žaidimo esybės lieka atskiros. Tai gali reikšti daugiau piešimo iškvietimų nei vienas pasaulio tinklas, todėl pagal numatymą išjungta. Atskiri OBJ, glTF ir GLB eksportai langelių nenaudoja.

## Pastabos apie tikslus
- Godot: tekstūrų ir medžiagų `res://` keliai, paleidiklių fizikos sluoksnis ir šviesos žemėlapio UV2. Eksporto aplanką pasirinkite Godot projekto viduje.
- Defold: išteklių katalogas, modelio medžiaga ir tekstūros ėmiklis, kuris turi egzistuoti pasirinktoje medžiagoje.
- Unity: OBJ tinklai ir redaktoriaus importuotojas; eksportuokite į projekto aplanką Assets.
- Unreal Engine: sugeneruotas importavimo skriptas ir turinio kelias, paprastai /Game/<level>.
- Flax Engine: modelio turinio kelias, santykinis projekto aplanko Content atžvilgiu.
- three.js, Babylon.js, PlayCanvas, Bevy, raylib ir Cocos Creator: scenos arba kodas toms vykdymo aplinkoms, su instrukcijomis šalia eksporto.
- glTF / GLB: perkeliama geometrija suderinamiems įrankiams; GLB yra dvejetainė forma. OBJ: geometrija ir medžiagos modeliavimo įrankiams.
- `.map`: **Įrašyti kaip** leidžia pasirinkti Valve 220, Standartinis, Quake 2 arba Quake 2 (Valve). Langas nurodo, ką prarastų pasirinktas dialektas.

Po įrašymo perskaitykite įspėjimus, pavyzdžiui, apie trūkstamas tekstūras. **Ką rado patikra** juos išvardija lange.

# Tiesioginiai ryšiai: Godot, Unity, Defold ir Flax
Tiesioginis ryšys jums dirbant siunčia neišsaugotus Heron Brush pakeitimus į variklio redaktorių. Jis vienkryptis: variklyje atlikti pakeitimai negrįžta. Viename žemėlapio lange gali veikti vienas tiesioginis ryšys. Sustabdykite jį prieš keisdami variklį ir paleiskite iš naujo, kad būtų pritaikytos pakeistos eksporto parinktys. Valdikliai yra eksporto lange, variklio puslapyje, kortelėje **Tiesioginis ryšys su**.

Indikatorius antraštės juostoje rodo ryšio būseną; spustelėkite jį, kad sustabdytumėte ryšį. Uždarius arba pakeitus žemėlapį, publikavimas sustoja. Po importavimo klaidos ištaisykite problemą ir vėl paleiskite ryšį.

## Godot
1. Vieną kartą eksportuokite į Godot projektą.
2. Spustelėkite **Įdiegti Godot priedą**.
3. Godot nustatymuose Project Settings → Plugins įjunkite Heron Brush Live.
4. Spustelėkite **Paleisti tiesioginį ryšį** ir iš Godot skydelio prijunkite žemėlapį prie atvertos 3D scenos.

Priedas pakeičia tik savo sugeneruotą šaką ir išlaiko prijungimo padėtį bei kitus mazgus. Godot sceną išsaugokite patys, o savo skriptus ir mazgus laikykite už sugeneruotos šakos ribų.

## Unity
1. Vieną kartą eksportuokite į Unity projekto aplanką Assets.
2. Spustelėkite **Įdiegti Unity priedą** ir leiskite Unity jį sukompiliuoti.
3. Unity programoje atverkite Window → Heron Brush → Live Link.
4. Paleiskite ryšį ir prijunkite žemėlapį prie aktyvios Unity scenos.

Laikykite Unity tiesioginio ryšio langą atvertą. Atnaujinimai pristabdomi Play Mode ir Prefab Mode režimuose. Pakeičiamas tik žemėlapio objekto vaikinis objektas Generated. Neredaguokite sugeneruotų failų.

## Defold
1. Vieną kartą eksportuokite į savo Defold projektą ir laikykite projektą atvertą Defold programoje.
2. Spustelėkite **Paleisti tiesioginį ryšį**. Jokio priedo nereikia.
3. Defold programoje atverkite heron_brush_live/preview.collection arba nurodykite jį savo scenoje.

Ryšys rašo į atskirą sugeneruotą aplanką ir nekeičia galutinio eksporto bei game.project. **Karštai perkrauti veikiantį žaidimą** taip pat atnaujina iš Defold paleistą žaidimą; pridėjus objektų ar pakeitus susidūrimus gali tekti iš naujo paleisti lygį. Nepavykę prisijungimai automatiškai kartojami.

## Flax
Eksportuokite į projekto aplanką Content ir spustelėkite **Įdiegti Flax priedą**. Leiskite Flax jį sukompiliuoti, paleiskite ryšį, tada atvertoje Flax scenoje pasirinkite Tools → Heron Brush: attach live map. Sugeneruota šaknis išlaiko savo padėtį, o kiti objektai nepaliečiami. Sceną išsaugokite patys. Atnaujinimai pristabdomi Play Mode režime. Tools → Heron Brush: stop live map pristabdo gavimą.

# Bendri tiesioginiai seansai
Bendradarbiavimas → Tiesioginis seansas atveria langą, kuriame galite rengti seansą kitam Heron Brush naudotojui arba prie jo prisijungti. Taip žemėlapį redaguoja keli žmonės; tai atskira nuo tiesioginio ryšio su varikliu.

## Rengimas ir prisijungimas
Įveskite **Jūsų vardas**. Norėdami rengti seansą, pasirinkite **Rengti seansą**, spustelėkite **Pradėti seansą** ir nusiųskite svečiams duomenis iš **Kopijuoti prisijungimo duomenis**. Norėdami prisijungti, pasirinkite **Prisijungti prie seanso**, įklijuokite duomenis ir spustelėkite **Sujungti**. Prisijungus atvertas žemėlapis pakeičiamas rengėjo žemėlapiu, todėl pirmiausia išsaugokite savo darbą.

Standartinė versija veikia vietiniame tinkle, o **Vietinio tinklo prievadas** nustato prievadą. Seansams internetu (**Jungtis per internetą**) reikia versijos su pasirenkama tinklo funkcija. Dalyviai 3D vaizde mato vienas kito kamerų žymes ir vardus.

Uždarius langą, seansas tęsiasi; antraštės juostos indikatorius rodo jo būseną. **Stabdyti seansą** arba **Palikti seansą** jį baigia. Kai du žmonės redaguoja tą patį objektą, galutinę versiją lemia rengėjo tvarka, o atšaukimas taip pat yra bendras pakeitimas. Kur įmanoma, dirbkite skirtingose srityse ir išsaugokite kontrolinius taškus.

# 3D rodymas ir antrasis monitorius
## Rodymo režimai
Kortelės Žemėlapis skyrius **Rodinys** nustato, ką piešia vaizdai: Brašai, Taškinės esybės, Esybių modeliai, Esybių klasių pavadinimai, Grupių kontūrai, Lopai, Medžiagos, Rodyti praleistas sieneles, Animacijų peržiūra, Vielinis karkasas, Briaunos ir Brašų esybių kontūrai. Vielinis karkasas piešia trikampių briaunas, įskaitant įstrižaines ir briaunas už paviršių, be medžiagų. Tai laikinas derinimo vaizdas, kuris neturi įtakos eksportams ar atvaizdavimams.

3D vaizdo apšvietimas, šešėliai, aplinkos okliuzija, atspindžiai, švytėjimas ir glotninimas nustatomi puslapyje Nuostatos → Atvaizdavimo variklis. Tai peržiūros nustatymai; galutiniai vaizdai naudoja atvaizdavimo kameros nustatymus.

## Atskiras 3D langas
Rodinys → 3D vaizdas atskirame lange perkelia 3D vaizdą į atskirą langą, kurį galite padėti kitame monitoriuje. Jis naudoja tą pačią kamerą, dokumentą ir įrankius, o plokšti vaizdai užima vietą pagrindiniame lange. Jo meniu siūlo Medžiagos, Vientisas vaizdas ir Vielinis karkasas; submeniu Rodinys su Rodyti praleistas sieneles, Animacijų peržiūra, Briaunos, Grupių kontūrai, Taškinės esybės, Esybių modeliai ir Brašų esybių kontūrai; Sufokusuoti į pažymėtus; Priekis, Šonas ir Viršus, kurie apsuka kamerą aplink pažymėtus objektus (arba tašką prieš kamerą); Padėti kamerą pagal 3D vaizdą; Visas ekranas (`F11`); Atvaizduoti vaizdą; ir Grįžti į pagrindinį langą. Uždarius langą, vaizdas vėl prisitvirtina.

## Procedūrinis dangus
Procedūrinis dangus 3D vaizde įjungtas pagal numatymą ir jį galima išjungti puslapyje Nuostatos → Atvaizdavimo variklis → Šviesa. Atvaizdavimo kameros turi savo jungiklį **Procedūrinis dangus**, išsaugomą kartu su žemėlapiu. Dangaus, horizonto ir žemės spalvos imamos iš aplinkos nustatymų, o saulė seka scenos apšvietimą. Dangus neturi geometrijos ir nėra eksportuojamas.

# Atvaizdavimo kameros ir PNG vaizdai
Failas → Atvaizduoti vaizdą atveria atvaizdavimo langą: kairėje kamera ir jos nustatymai, dešinėje tiesioginė peržiūra, kuri persipiešia jums juos keičiant. Kameros išsaugomos `.map` faile, tik redaktoriui skirtame sluoksnyje Render cameras, kuris neįtraukiamas į eksportą į variklius.

## Kameros padėjimas
Sukomponuokite kadrą 3D vaizde ir paspauskite `Ctrl+Alt+0` (arba `Ctrl+Alt+Num0`) arba spustelėkite **Padėti kamerą pagal 3D vaizdą**. Jei kamera nepažymėta, atnaujinama pirmoji išsaugota; jei jų nėra, sukuriama nauja. Tai galima atšaukti, o išvesties ir apšvietimo nustatymai išlieka. **Pažymėti kamerą žemėlapyje**, **Įkelti kamerą iš žemėlapio** ir **Perkelti 3D vaizdą į kamerą** veikia priešinga kryptimi. **Kameros rėmas 3D vaizde** rodo išvesties ribą, o **Valdyti kamerą iš 3D vaizdo** leidžia įprastu naršymu judinti išsaugotą kamerą.

## Nustatymai
- **Kameros padėtis**: X, Y, Z, posūkis ir polinkis.
- **Išvestis ir objektyvas**: **Išankstiniai nustatymai** (Full HD, Kvadratas, Stačias), plotis, aukštis, imtys, **Vertikalus matymo laukas** ir **Tolimoji kirpimo riba**.
- **Apšvietimas**: Dienos šviesa, Saulėlydis arba Debesuota, arba nustatykite ekspoziciją, saulės, dangaus, horizonto ir žemės spalvas, **Procedūrinis dangus**, **Šešėliai**, **Aplinkos okliuzija** ir **Švytėjimas**.
- **Lauko gylis**: **Įjungti lauko gylį**, fokuso atstumas, ryškumo sritis ir suliejimo spindulys. **Sufokusuoti į pažymėtus** sufokusuoja į pažymėto objekto prieš kamerą centrą.

**Išsaugoti kamerą ir nustatymus** įrašo formą į žemėlapį; išsaugokite žemėlapį, kad ji liktų diske.

## Atvaizdavimas
**Atvaizduoti PNG** paklausia, kur išsaugoti, ir pradeda laipsnišką atvaizdavimą, kuris peržiūroje pildosi imtis po imties. Daugiau imčių atrodo geriau, bet užtrunka ilgiau. **Atšaukti atvaizdavimą** sustabdo neįrašydamas failo. Atvaizdavimo variklis naudoja redaktoriaus rastrinį konvejerį, o ne spindulių sekimą (path tracing). PNG turi nepermatomą foną, o animacijos neatvaizduojamos. Labai dideli vaizdai gali trumpam pristabdyti redaktorių.

# Nuostatos
Failas → Nuostatos (`Ctrl+,`) atveria nustatymus. Pakeitimai taikomi iškart ir išsaugomi. Šoninėje juostoje išvardyti puslapiai Apžvalga, Žaidimai, Rodinys, Spalvos, Pelė, Klaviatūra ir Atvaizdavimo variklis. Paieškos laukas viršuje randa nustatymą pagal pavadinimą arba aprašymą bet kuriame puslapyje. Kiekvienas pakeistas nustatymas turi savo mygtuką numatytajai reikšmei atkurti, o **Atkurti numatytuosius** atstato dabartinį puslapį.

## Puslapiai
- **Apžvalga**: dažniausi nustatymai kartu: Išvaizda (Skydelių nepermatomumas (%), Spalvų schema, Išdėstymas, Kalba), Redagavimas, Rodinys ir Vadovas.
- **Žaidimai**: žaidimas, kur yra jo failai, ir jūsų vardas bendradarbiaujant. Žaidimas nurodo, kokiu žemėlapio formatu rašyti, kokius esybių apibrėžimus įkelti ir kur yra jo medžiagos. Žaidimų konfigūracijos skaitomos iš aplanko `games` konfigūracijos aplanke (Linux ir macOS sistemose `~/.config/heron-brush/games`, Windows – `%LOCALAPPDATA%\heron-brush\games`) ir iš aplanko `games` šalia programos, į kurį paketai įdeda pridedamą konfigūraciją.
- **Rodinys**: Išdėstymas, Spalvų schema, Kalba, pasaulio ašys, tinklelio nepermatomumas, nuotėkių tikrinimo langelio dydis ir išdėstymo užraktai.
- **Spalvos**: vaizdų, geometrijos ir rankenėlių spalvos.
- **Pelė**: apsidairymo, slinkimo ir judėjimo greičiai bei apvertimai, skrydžio greitis, matymo laukas, Susieti 2D vaizdus ir rankenėlių dydžiai.
- **Klaviatūra**: klavišų schema, Heron Brush arba Blender.
- **Atvaizdavimo variklis**: realistiškas šešėliavimas, žymėjimo kontūras, šviesa, šešėliai, aplinkos okliuzija, atspindžiai, švytėjimas ir paviršiai.

## Kalba
**Kalba** siūlo Kaip darbalaukyje ir kiekvieną išverstą kalbą. Sąsaja pasikeičia iškart. Vadovas atsiveria sąsajos kalba, jei yra jo vertimas, kitaip – anglų kalba.

# Papildiniai ir MCP
## Papildiniai
Papildiniai praplečia redaktorių Rhai skriptais. Failas → Papildiniai atveria papildinių langą. **Įdiegti papildinį** įdiegia `.rhai` failą, o **Įdiegti papildinio aplanką** įdiegia aplanką su `main.rhai`; pasirenkami `plugin.toml` metaduomenys aprašo papildinį ir jo nustatymus. Papildiniai sugrupuoti į **Aktyvūs papildiniai** ir **Neaktyvūs papildiniai**, o **Įjungti papildinį** jį įjungia arba išjungia; tai išlieka ir po paleidimo iš naujo. **Perkrauti papildinius** iš naujo nuskaito skriptus iš disko ir atstato jų skydelių reikšmes.

Aktyvūs papildiniai taip pat rodomi kortelės Žemėlapis skyriuje **Papildiniai** su savo nustatymais, veiksmų mygtukais (**Vykdyti**) ir išvestimi. Veiksmas veikia su pažymėtais objektais arba, jei niekas nepažymėta, su viskuo, ir yra vienas atšaukimo žingsnis. Pridėtas žymėjimo inspektorius rodo kiekius ir veiksmus perkelti aukštyn ir žemyn.

## MCP
Bendradarbiavimas → Teikti agentui (MCP) leidžia DI agentui ar kitam įrankiui valdyti atvertą redaktorių per Model Context Protocol. Langas rodo vietinį adresą su **Kopijuoti**, o **Stabdyti serverį** jį baigia; kol jis veikia, antraštės juostoje rodoma **MCP aktyvus**. Komandų eilutės parinktis `--mcp` paleidžia tą pačią sąsają be lango. Klientai gali aptikti įrankius geometrijai, ištekliams, savybėms, animacijai, eksportui ir seansams. MCP ir redaktoriaus papildiniai yra skirtingos sąsajos.

# Komandų eilutė
Programa vadinasi `heron-brush`. Be parinkčių ji atveria pasveikinimo ekraną.

## Failų atvėrimas
`heron-brush map.map [entities.fgd] [materials/]` atveria žemėlapį, pasirinktinai su esybių apibrėžimais ir medžiagų aplanku. Tuščias argumentas vieną praleidžia, pavyzdžiui, `heron-brush map.map "" materials/`. Jei redaktorius jau veikia, žemėlapis atveriamas jame; `--new-instance` vietoj to atveria atskirą langą.

## Komandos
- `--help` arba `-h`: išspausdinti naudojimo informaciją. `--version` arba `-V`: išspausdinti versiją.
- `--mcp`: teikti MCP per standartinę įvestį ir išvestį, be lango.
- `--merge-map <base> <ours> <theirs> [<output>]`: trikryptis `.map` failų suliejimas git sistemai. Išvestis pagal numatymą yra ours. Konfliktai išspausdinami ir grąžinamas nenulinis išėjimo kodas.
- `--obj-to-map <in.obj> <out.map> [size] [thickness] [snap] [material]`: paversti tinklą brašais. Size yra ilgiausios kraštinės ilgis žemėlapio vienetais (numatytasis 256), snap yra kampų tinklelis (0 – be jo), o žodžiai `solid` arba `layered` vietoj plonų pleištų sukuria tinklelio dėžes arba iškiląsias plokštes.
- `--render-preview <in.map> <out.png> [size]`: įrašyti žemėlapio PNG miniatiūrą, pagal numatymą 512 pikselių.
- `--run-script <script.rhai> <in.map> [out.map] [key=value ...]`: paleisti papildinio skriptą žemėlapiui, kai viskas pažymėta, ir įrašyti rezultatą (pagal numatymą atgal į įvesties failą). `key=value` atsako į papildinio nustatymus.

# Trikčių šalinimas
## Kai kas nors atrodo ne taip
- Trūksta medžiagos: patikrinkite medžiagų aplanką ar paketą ir eksporto projekto kelius.
- Netinkamas modelio dydis: palyginkite su 64 vienetų žaidėjo žymekliu ir eksporto masteliu.
- Netinkama gradiento kryptis: patikrinkite gradiento kampą ir sienelės orientaciją.
- Tuščia variklio scena: perskaitykite eksporto įspėjimus ir vadovaukitės tikslo importavimo instrukcijomis.
- Tiesioginis ryšys laukia: patikrinkite, ar priedas įdiegtas, scena prijungta ir atnaujinimai nepristabdyti.
- Tik linijos, be sienelių: išjunkite Vielinis karkasas kortelės Žemėlapis skyriuje Rodinys arba 3D lango meniu pasirinkite Medžiagos ar Vientisas vaizdas.
- Vaizduose trūksta objektų: `Ctrl+Shift+I` parodo paslėptus objektus; patikrinkite sluoksnius ir View filters.
- Klavišas nieko nedaro: pirmiausia spustelėkite vaizdą, kad jis gautų klaviatūrą, ir patikrinkite klavišų schemą puslapyje Nuostatos → Klaviatūra.
- Pakeitimas nepavyko: atšaukite, supaprastinkite geometriją ir naudokite **Tikrinti žemėlapį** bei **Tikrinti nuotėkius** kortelės Žemėlapis skyriuje **Problemos**.
- Žemėlapis neatsiveria iš failų tvarkyklės: jis galėjo būti perduotas jau veikiančiam redaktoriui; ieškokite to lango.

## Nustatymai ir atsarginės kopijos
Nuostatos, paskutiniai žemėlapiai, žvaigždutės, eksporto nustatymai ir papildiniai saugomi konfigūracijos aplanke: Linux ir macOS sistemose `~/.config/heron-brush`, o Windows – `%LOCALAPPDATA%\heron-brush`. Automatinio išsaugojimo kopijos yra aplanke `autosave` šalia kiekvieno žemėlapio.

## Pranešimas apie klaidą
Nurodykite redaktoriaus versiją (Pagalba → Apie), savo operacinę sistemą, nedidelį žemėlapį, kuriame matoma problema, ir tikslius veiksmus. Išvesties sritis (`Ctrl+4`) dažnai parodo, kas nutiko. Prieš eksperimentuodami išsaugokite kopiją.

## Kaip naudotis šiuo vadovu
Turinyje kairėje pasirinkite skyrių. `Ctrl+F` parodo turinį ir perkelia į paieškos lauką; `Esc` išvalo paiešką arba, jei paieška tuščia, uždaro vadovą. **Turinys** paslepia arba parodo skyrių sąrašą, o **Teksto dydis** keičia šrifto dydį; abu nustatymai įsimenami. Kalbų sąrašas viršuje perjungia prieinamus vertimus.
