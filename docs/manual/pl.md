# Pierwsze kroki
Heron Brush to edytor poziomów oparty na brushach. Poziomy budujesz z wypukłych brushów, malujesz i teksturujesz ich ściany, rozmieszczasz encje, animujesz drzwi i platformy, a wynik eksportujesz do silnika gry. Edytor ma też wbudowane assety, kształty parametryczne, połączenia na żywo z edytorami silników, wspólne sesje edycji i renderowanie statycznych obrazów. Ten podręcznik jest dołączony do aplikacji i działa bez internetu.

Na macOS `Cmd` działa wszędzie tam, gdzie podręcznik wymienia `Ctrl`, a Option odpowiada klawiszowi Alt.

## Ekran powitalny
Gdy Heron Brush uruchamia się bez mapy, otwiera się ekran powitalny. Po lewej są **Kontynuuj** (ostatnio otwarta mapa), **Nowa mapa** i **Przeglądaj** do otwierania istniejącego pliku `.map`, a także **Gra**, **Podręcznik** i **Preferencje**. Po prawej znajduje się lista ostatnich map z miniaturami, a pod nią **Przykłady**. Gwiazdka (**Przypnij na górze**) przypina mapę na początku listy; kosz (**Przenieś do kosza**) przenosi plik do systemowego kosza, skąd można go przywrócić. Klawisze strzałek poruszają się po ekranie, a Enter otwiera zaznaczoną pozycję. Plik → Powitanie w każdej chwili przywraca ten ekran.

## Twoja pierwsza mapa
1. Wybierz **Nowa mapa** na ekranie powitalnym albo Plik → Nowa mapa (`Ctrl+N`).
2. Wybierz na pasku narzędzi narzędzie Brush (`B`) i przeciągnij w widoku 2D, aby narysować wybrany kształt. Sprawdź jego głębokość w innym widoku 2D.
3. Zaznacz brush i przesuń go, przeciągając. Na karcie **Ściana** nadaj mu materiał lub kolor.
4. Zapisz mapę źródłową skrótem `Ctrl+S`. Gdy zechcesz użyć jej w grze, otwórz Plik → Eksport i zapisz scenę dla swojego silnika.

## Pliki i zapisywanie
Plik `.map` to twoje edytowalne źródło. Zapisuj regularnie: eksport, połączenia na żywo i renderowanie nigdy nie zapisują mapy za ciebie. Plik → Zapisz jako (`Ctrl+Shift+S`) dopisuje rozszerzenie `.map`, jeśli je pominiesz. Plik → Wróć do zapisanej mapy odrzuca niezapisane zmiany i wczytuje plik ponownie. Jeśli zamykasz okno albo otwierasz inną mapę, mając niezapisane zmiany, Heron Brush zapyta, czy **Zapisz**, **Odrzuć** czy **Anuluj**.

Dopóki zapisana mapa ma niezapisane zmiany, co dziesięć minut w folderze `autosave` obok mapy powstają numerowane kopie zapasowe. Przechowywanych jest 50 najnowszych.

Jeśli otworzysz mapę, gdy Heron Brush już działa, trafi ona do działającego okna, które wysunie się na pierwszy plan. Aby otworzyć osobne okno, użyj w wierszu poleceń `--new-instance` (zob. Wiersz poleceń).

# Interfejs
Główne okno ma pasek tytułu z menu, pod nim pasek narzędzi, widoki mapy, inspektor po prawej i pasek stanu u dołu.

## Pasek tytułu i menu
Pasek tytułu zawiera logo, menu, nazwę mapy pośrodku i przyciski okna. Na macOS zwykłe czerwony, żółty i zielony przycisk znajdują się po lewej. Przeciągnij pasek tytułu, aby przesunąć okno, a kliknij go dwukrotnie, aby je zmaksymalizować. Gdy coś działa w tle, przy końcu paska pojawia się wskaźnik: połączenie na żywo z silnikiem, sesja na żywo lub **MCP aktywny**. Kliknij wskaźnik, aby zatrzymać połączenie na żywo albo otworzyć jego okno.

- **Plik**: Powitanie, Nowa mapa (`Ctrl+N`), Otwórz mapę (`Ctrl+O`), Zapisz (`Ctrl+S`), Zapisz jako (`Ctrl+Shift+S`), Wróć do zapisanej mapy, Eksport, Renderuj obraz, Wtyczki i Preferencje (`Ctrl+,`).
- **Edycja**: Cofnij (`Ctrl+Z`), Ponów (`Ctrl+Shift+Z`), Wytnij, Kopiuj, Wklej, Duplikuj (`Ctrl+D`), Usuń, Zaznacz wszystko (`Ctrl+A`), Powtórz ostatnie polecenia (`Ctrl+R`) i Zapomnij powtarzalne polecenia.
- **Widok**: Widok 3D w osobnym oknie, Przejdź do, Przyciągaj do siatki (`Alt+0`), Pokaż siatkę (`0`), Podgląd animacji i Pokaż pominięte ściany.
- **Geometria**: Wstaw tekst 3D, Połącz narożnik, Zamień ściany na patche, Wyjmij zaznaczenie z jego encji i Zsynchronizuj połączone grupy.
- **Animacja**: Animacja klatkami kluczowymi, Podgląd animacji i Umieść kamerę według widoku 3D (`Ctrl+Alt+0`).
- **Mapa**: Wczytaj plik punktów, Wczytaj ponownie plik punktów i Zamknij plik punktów; Wczytaj plik portali, Wczytaj ponownie plik portali i Zamknij plik portali. Pokazują ślady nieszczelności i portale zapisane przez kompilatory map.
- **Współpraca**: Sesja na żywo i Udostępnij agentowi (MCP).
- **Pomoc**: Podręcznik i O programie.

## Pasek narzędzi
Na pasku narzędzi są wszystkie narzędzia. Pierwszej grupy używa się bez przerwy: Zaznacz, Brush, Cięcie, Wierzchołek, Krawędź, Ściana, Obróć i Skala. Druga grupa zawiera Pochyl, Złóż, Wyciąganie, Punkt kontrolny, Zaokrąglenie i Malowanie. Najedź kursorem na przycisk, aby zobaczyć jego nazwę i skrót. Ponowne kliknięcie aktywnego narzędzia przywraca narzędzie Zaznacz.

Za narzędziami są **Odbij w poziomie** i **Odbij w pionie**, a potem dwa przełączniki: **Blokada** zachowuje mapowanie materiału przy przesuwaniu brusha, a **UV** zachowuje je przy jego obracaniu (`U` go przełącza). Dalej są ustawienia aktywnego narzędzia: narzędzie Brush pokazuje kształt i jego opcje, a narzędzie Zaokrąglenie promień, liczbę ścian i narożniki z przyciskiem **Zastosuj**. Na prawym końcu są nazwa narzędzia, rozmiar siatki (**Siatka 16** itd.; kliknij, aby wybrać inny) i menu **Ustawienia siatki** z pozycjami Przyciągaj do siatki, Pokaż siatkę, Podgląd animacji, Pokaż pominięte ściany i Animacja klatkami kluczowymi.

## Widoki mapy
Są cztery widoki: widok 3D i trzy płaskie widoki: Z góry (XY), Z przodu (XZ) i Z boku (YZ). Preferencje → Widok → Układ pozwala wybrać Jeden panel, Dwa panele, Trzy panele lub Cztery panele. Przeciągaj przegrody, aby zmienić rozmiar paneli. Kliknięcie w widok przekazuje mu klawiaturę. `Space` przełącza panel, który pokazuje po jednym widoku naraz, na następny widok, a `Ctrl+Space` maksymalizuje widok, który ma klawiaturę, i z powrotem przywraca układ.

Kliknij prawym przyciskiem bez przeciągania w dowolnym widoku, aby otworzyć menu kontekstowe dla zaznaczenia i tego, co jest pod kursorem. Zawiera ono grupowanie, warstwy, połączone grupy, tworzenie encji, przenoszenie brushów do encji i z niej, kopiowanie nazwy materiału i pokazanie go w przeglądarce materiałów.

## Inspektor
Inspektor po prawej ma cztery karty. Przeciągnij jego lewą krawędź, aby zmienić szerokość.

- **Mapa** (`Ctrl+1`): Warstwy, Grupy, Wtyczki, Problemy, filtry sekcji Widok i Znaczniki.
- **Encja** (`Ctrl+2`): przeglądarka encji, definicje encji, właściwości zaznaczonej encji, a dla przycisków i wyzwalaczy także Wyzwalacz animacji.
- **Ściana** (`Ctrl+3`): UV, Materiały, Kolor, Wyciąganie, Zaokrąglenie, Patch oraz Wcięcie i faza. Każdą sekcję można zwinąć.
- **Assety**: biblioteka modeli z brushów do wielokrotnego użytku.

`Ctrl+5` ukrywa lub pokazuje inspektor, a `Ctrl+Alt+T` ukrywa lub pokazuje pasek narzędzi, co daje widokom więcej miejsca.

## Pasek stanu i panel komunikatów
Pasek stanu u dołu pokazuje ostatni komunikat edytora. Błędy są wyświetlane na czerwono. Kliknij pasek albo naciśnij `Ctrl+4`, aby otworzyć panel komunikatów z pełną historią. Większość poleceń zgłasza tu, co zrobiła, na przykład ile brushów zostało przyciętych.

## Okna dialogowe i wygląd
Okna dialogowe, takie jak Preferencje, Eksport i Renderuj obraz, otwierają się jako osobne okna. Preferencje → Widok → Schemat kolorów pozwala wybrać Zgodnie z systemem, Jasny lub Ciemny. **Krycie paneli (%)** na stronie Przegląd sprawia, że panele prześwitują nad widokami. Okno rysuje własne przyciski w stylu systemu: natywne na macOS, w stylu Windows 11 na Windows i w stylu GNOME na Linuksie.

# Nawigacja i klawiatura
## Poruszanie się w widoku 3D
Przytrzymaj prawy przycisk myszy i przeciągaj, aby się rozglądać. Przeciąganie środkowym przyciskiem przesuwa widok. Kółko przesuwa kamerę do przodu i do tyłu. `W`, `A`, `S` i `D` lecą do przodu, w lewo, do tyłu i w prawo, `Q` wznosi, a `X` opuszcza kamerę; przytrzymaj `Shift`, aby robić większe kroki. Gdy w Preferencje → Mysz włączona jest opcja **Alt + przeciąganie środkowym przyciskiem porusza kamerą**, Alt ze środkowym przyciskiem przesuwa kamerę wzdłuż kierunku patrzenia. Prędkości, odwrócenia osi i pole widzenia znajdziesz w Preferencje → Mysz.

## Poruszanie się w widokach 2D
Przeciągaj środkowym przyciskiem, aby przesuwać widok, a kółkiem przybliżaj wokół kursora. Przesuwanie i przybliżanie jednego płaskiego widoku utrzymuje pozostałe w zgodzie; steruje tym opcja **Powiąż widoki 2D** w Preferencje → Mysz. `Ctrl+Alt+Z` resetuje przybliżenie, a `Ctrl+Shift+U` ustawia płaskie widoki z powrotem na początku układu współrzędnych.

`Ctrl+U` kadruje zaznaczenie we wszystkich widokach. Widok → Przejdź do (`Ctrl+Alt+M`) przenosi kamerę do wpisanych współrzędnych, zaznacza to, co jest w danym wierszu pliku mapy, albo przesuwa zaznaczenie o wpisaną wartość.

## Siatka i przyciąganie
`+` i `-` zmieniają rozmiar siatki, a klawisze cyfr od `1` do `9` wybierają rozmiar bezpośrednio, od 1 do 256 jednostek. `0` lub `#` pokazuje albo ukrywa siatkę, a `Alt+0` włącza lub wyłącza przyciąganie. `S` przyciąga wierzchołki zaznaczonych brushów do bieżącej siatki, `Ctrl+Shift+V` do całych jednostek, a `Ctrl+Alt+Shift+V` do siatki.

## Najczęstsze skróty
- `Ctrl+S`: zapisz mapę. `Ctrl+Shift+S`: zapisz jako.
- `Ctrl+Z`: cofnij. `Ctrl+Shift+Z` lub `Ctrl+Y`: ponów.
- `Ctrl+C`, `Ctrl+X`, `Ctrl+V`: kopiuj, wytnij i wklej. Wklejanie umieszcza obiekty przy kursorze; `Ctrl+Alt+V` zachowuje ich pierwotne współrzędne. Gdy zaznaczone są ściany, kopiowanie pobiera ich materiał i ustawienia UV.
- `Ctrl+D`: duplikuj. `Ctrl+Shift+D`: połączony duplikat grupy.
- `Delete` lub `Backspace`: usuń zaznaczenie.
- `Esc`: anuluj bieżące przeciąganie, cofnij się o krok w aktywnym narzędziu, wróć do narzędzia Zaznacz, a na końcu wyczyść zaznaczenie. `Shift+Esc` wraca do narzędzia Zaznacz od razu.
- `Ctrl+R`: powtórz ostatnie powtarzalne polecenia. `Ctrl+Shift+R`: zapomnij je.
- `Ctrl+Alt+0` lub `Ctrl+Alt+Num0`: ustaw kamerę renderowania w miejscu widoku 3D.
- `F5`: wczytaj ponownie materiały. `F6`: wczytaj ponownie definicje encji.

## Skróty zaznaczania
- `Ctrl+A`: zaznacz wszystko albo, gdy aktywne jest narzędzie wierzchołków, krawędzi lub ścian, wszystkie uchwyty. `Ctrl+Shift+A`: odznacz wszystko. `Ctrl+Alt+A`: odwróć zaznaczenie.
- `Ctrl+B`: zaznacz pozostałe obiekty w tej samej encji lub grupie.
- `Ctrl+T`: zaznacz wszystko, czego dotykają zaznaczone brushe. `Ctrl+E`: zaznacz wszystko, co znajduje się w ich wnętrzu. `Ctrl+Shift+E`: zaznacz wszystko w ich kolumnie wzdłuż osi widoku. Brushe użyte do zaznaczania zostają przy tym zużyte.
- `Ctrl+I`: izoluj zaznaczenie. `Ctrl+Alt+I`: ukryj je. `Ctrl+Shift+I`: pokaż znowu wszystko.
- `Ctrl+G`: grupuj. `Ctrl+Shift+G`: rozgrupuj.

## Skróty przekształceń
- Klawisze strzałek oraz `Page Up` / `Page Down` przesuwają zaznaczenie o jeden krok siatki w kierunku, który wskazują w aktywnym widoku. Z `Ctrl` duplikują podczas przesuwania, a z `Alt` obracają.
- `Alt+X`, `Alt+Y`, `Alt+Z`: obrót o ćwierć obrotu wokół danej osi; dodaj `Shift`, aby obrócić w przeciwną stronę.
- `Ctrl+F`: odbij w poprzek widoku; `Ctrl+Alt+F`: odbij w pionie. `Ctrl+Shift+X` i `Ctrl+Shift+Y` tworzą odbicie lustrzane wzdłuż X lub Y.
- `Insert`: umieść encję punktową w środku zaznaczenia.
- `Tab`: gdy zaznaczony jest brush, przełączaj kolejno Zaznacz → Wierzchołek → Krawędź → Ściana → Zaznacz.

## Układ skrótów Blendera
Preferencje → Klawiatura → Układ skrótów oferuje Heron Brush (domyślny, używany w całym tym podręczniku) i Blender. W układzie Blender: `G` zaznacza i przesuwa, `R` obraca, `S` skaluje, `E` wyciąga, `1`, `2` i `3` wybierają narzędzia wierzchołków, krawędzi i ścian, `K` tnie, `B` zaznacza, `A` zaznacza wszystko, a `Alt+A` usuwa zaznaczenie, `X` usuwa, `Shift+D` duplikuje, `H` ukrywa, a `Alt+H` pokazuje wszystko, zaś `Ctrl+Shift+Z` ponawia. Wszystko inne działa tak samo w obu układach.

# Brushe i narzędzia modelowania
Brushe to wypukłe bryły wyznaczone przez płaszczyzny. Złożone obiekty buduj z kilku prostych brushów, zamiast próbować zrobić jeden brush wklęsły. Tam, gdzie wystarczy prosty kształt, utrzymuj małą liczbę brushów.

## Rysowanie brushów
Wybierz narzędzie Brush (`B`) i przeciągnij w widoku. Pasek narzędzi pokazuje **Rysowany kształt**: Prostopadłościan, Schody, Łuk (sklepienie), Walec, Stożek, Sferoida (UV), Sferoida (dwudziestościan) i Wstaw tekst 3D. `Alt+1` do `Alt+7` wybierają kształt bezpośrednio. Zależnie od kształtu pasek pokazuje **Wydrąż** i **Grubość:**, dla schodów **Wysokość stopnia:** i **Kierunek:**, dla kształtów obłych **Oś:**, **Boki:** i tryb okręgu, dla sfer **Pierścienie:**, a dla dwudziestościanu **Dokładność:**. Ustaw opcje przed przeciągnięciem. Sprawdź wynik w więcej niż jednym widoku.

## Zaznaczanie
Kliknij brush, aby go zaznaczyć, a kliknij z `Ctrl`, aby dodać go do zaznaczenia. Przeciągnięcie od pustego miejsca rysuje prostokąt zaznaczenia. W widoku 3D kliknięcie z `Shift` zaznacza pojedynczą ścianę, a kliknięcie z `Ctrl+Shift` dodaje kolejne ściany. Przeciąganie zaznaczonego brusha go przesuwa; przeciągnięcie niezaznaczonego zaznacza i przesuwa tylko ten jeden.

W widoku 3D przeciągnij z `Shift` ścianę zaznaczonego brusha, aby ją wyciągnąć. Kliknij z `Alt` inną ścianę, aby skopiować na nią materiał i wyrównanie zaznaczonej ściany; `Alt+Ctrl` kopiuje tylko materiał, a `Alt+Shift` kopiuje obrót.

## Narzędzia kształtowania
- **Zaznacz** (`Esc`): zaznaczanie i przesuwanie.
- **Wierzchołek** (`V`), **Krawędź** (`E`), **Ściana** (`F`): przeciąganie narożników, krawędzi lub ścian. Przeciągnięcie z `Shift` od środka krawędzi lub ściany dodaje w tym miejscu narożnik. `Delete` usuwa wybrane narożniki.
- **Obróć** (`R`), **Skala** (`T`), **Pochyl** (`G`): przekształcanie zaznaczenia za pomocą uchwytów.
- **Cięcie** (`C`): kliknij dwa lub trzy punkty, aby wyznaczyć płaszczyznę cięcia. `Enter` tnie, `Ctrl+Enter` wybiera, którą stronę zachować, a `Backspace` lub `Esc` usuwa ostatni punkt.
- **Złóż** (`N`): rozmieść punkty wokół bryły i naciśnij `Enter`, aby utworzyć z nich wypukły brush. `Backspace` usuwa ostatni punkt.
- **Wyciąganie** (`Y`): zob. sekcję o wyciąganiu poniżej.
- **Punkt kontrolny** (`P`): zmiana kształtu siatki kontrolnej patcha.
- **Zaokrąglenie** (`O`): kliknij krawędź, aby ją zaokrąglić, albo wybierz krawędzie lub wierzchołki ich narzędziami i naciśnij `Enter`. Więcej ścian daje gładszy wynik, ale też więcej geometrii.
- **Malowanie** (`K`): zob. Materiały i UV.

## Łączenie brushów
- `Ctrl+J`: scal zaznaczone brushe w ich otoczkę wypukłą.
- `Ctrl+K`: odejmij zaznaczone brushe od brushów, których dotykają.
- `Ctrl+Shift+K`: wydrąż zaznaczenie, ze ściankami o grubości jednego kroku siatki.
- `Ctrl+L`: część wspólna zaznaczenia.

## Wyciąganie, zaokrąglenie, patch i faza
**Wyciąganie** tworzy geometrię wzdłuż ścieżki z zaznaczonych ścian. Zaznacz ściany (w narzędziu Wyciąganie kliknięciem z `Alt`), a potem w sekcji Wyciąganie karty Ściana ustaw **Przesunięcie**, **Obrót** (oś i kąt), **Skala**, **Kroki** i **Ścieżka** (wyrównanie Całkowite lub Dowolne; Łuk, Prosta lub Łuk S) i naciśnij **Wyciąganie**. Wiersz pod kontrolkami podaje, ile brushów powstanie.

Sekcja **Zaokrąglenie** zaokrągla wybrane krawędzie z parametrami **Promień**, **Ściany** i **Narożniki** (Na ucios lub Zaokrąglone); jedna ściana daje fazkę.

**Patch** pokazuje **Kolumny** i **Wiersze** zaznaczonego patcha wraz z przyciskami do dodania lub usunięcia pary. Geometria → Zamień ściany na patche (`Ctrl+P`) tworzy patche z zaznaczonych ścian.

**Wcięcie i faza** działa na zaznaczonych ścianach: **Wcięcie** cofa ścianę od jej krawędzi do środka, a **Głębokość** wypycha ją na zewnątrz (albo do wewnątrz, gdy jest ujemna). **Sfazuj ściany** stosuje oba; trzeci przycisk dodaje panel lub wycina wnękę, zależnie od znaku głębokości. **Połącz narożnik** otwiera okno, które łączy dwa brushe narożnikiem na ucios lub zaokrąglonym, z podglądem.

## Tekst 3D
Geometria → Wstaw tekst 3D (także ostatnia pozycja listy kształtów narzędzia Brush) buduje brushe z tekstu. Używa Heron Block, wbudowanej czcionki wielokątnej ze ściętymi narożnikami, proporcjonalnymi szerokościami i czeskimi znakami diakrytycznymi; małe litery mają postać wielkich. Wpisz **Tekst**, ustaw **Wysokość** (wysokość wielkich liter; znaki diakrytyczne wystają ponad nią) i **Głębokość**, obserwuj podgląd na żywo i naciśnij **Wstaw**.

Jeśli operacja nie może utworzyć poprawnej geometrii, zmień jej wymiary albo cofnij ją i uprość kształt.

## Szyki
Geometria → Szyk kopiuje zaznaczenie i pokazuje kopie na mapie, gdy zmieniasz wartości. Liczby obejmują oryginał, a szyk tworzy najwyżej 1024 kopie.

- **Rząd** przesuwa każdą kopię względem poprzedniej o **Krok X**, **Krok Y** i **Krok Z** i obraca ją o **Obrót na kopię (stopnie)** wokół wybranej osi.
- **Siatka** wypełnia **Liczba X**, **Liczba Y** i **Liczba Z** elementów w odstępach **Krok X**, **Krok Y** i **Krok Z**.
- **Pierścień** rozkłada elementy w zakresie **Kąt (stopnie)** wokół środka oddalonego o **Promień (jednostki)** od zaznaczenia. Z **Zwróć ku środkowi** każda kopia obraca się razem z pierścieniem.
- **Rozrzut** umieszcza kopie w losowych miejscach w promieniu, w poprzek wybranej osi. To samo **Ziarno** daje ten sam układ, a **Losowy obrót** obraca każdą kopię.

Gdy zaznaczona jest jedna cała grupa, **Połączone kopie** robi z każdej kopii połączony duplikat, więc zmiana jednej dociera do wszystkich. **Zastosuj** zostawia kopie i zaznacza je razem z oryginałem; **Anuluj** je usuwa. Cały szyk cofa się jednym krokiem.

## Rzeźbienie terenu
Teren składa się z wielu małych brushy, których wierzchy pędzel rzeźbiarski podnosi i opuszcza. Zaznacz płaski prostopadłościan i wybierz Geometria → Podziel na słupy: zostanie pocięty na trójkątne słupy szerokie na jeden krok siatki, umieszczone w grupie o nazwie Terrain. Wierzch trójkąta jest zawsze płaski, więc słupy mogą tworzyć grzbiety i doliny.

Wybierz narzędzie **Rzeźbienie** (`J`) i przeciągaj po terenie w widoku 3D. Pomarańczowy pierścień pokazuje pędzel. Bez zaznaczenia pędzel zmienia każdy brush pod sobą, z zaznaczeniem tylko zaznaczone. Pasek u góry okna wybiera, co robi pociągnięcie:

- **Podnieś** i **Obniż** przesuwają powierzchnię w górę lub w dół, najmocniej w środku pierścienia, a na krawędzi wcale. Przytrzymaj `Shift`, aby je zamienić.
- **Wygładź** wyrównuje nierówności.
- **Wyrównaj** wyrównuje powierzchnię do wysokości, na której zaczęło się pociągnięcie.

Dwie liczby obok trybów to **Promień (jednostki)** i **Siła (%)**. Przesuwają się tylko górne narożniki, pionowo; spód zostaje na miejscu, a słup ma co najmniej jedną jednostkę grubości. Pociągnięcie cofa się jednym krokiem. W widokach 2D narzędzie zaznacza tak jak narzędzie Zaznaczanie.

# Materiały i UV
## Wczytywanie i nakładanie materiałów
Otwórz sekcję **Materiały** na karcie Ściana. **Wczytaj folder** dodaje folder obrazów, a **Wczytaj .wad** dodaje pakiet tekstur; oba pojawiają się w **Kolekcje**, gdzie krzyżyk usuwa wybraną pozycję. Wpisz tekst w **Filtruj materiały**, aby zawęzić listę. Kliknij materiał, aby uczynić go bieżącym, a **Zastosuj do zaznaczenia** nałoży go na wszystkie zaznaczone ściany. **Zaznacz** oferuje Zaznacz brushe z tym materiałem i Zaznacz ściany z tym materiałem. **Zastąp** zamienia jeden materiał na inny w całej mapie lub tylko w zaznaczeniu.

Narzędzie **Malowanie** (`K`) nakłada bieżący materiał na ścianę pod kursorem: kliknięcie maluje jedną ścianę, dwukrotne kliknięcie cały brush, a kliknięcie z `Shift` pobiera materiał ściany jako bieżący, jak pipeta.

## Edytor UV
Sekcja **UV** na karcie Ściana pokazuje materiał i obrys zaznaczonej ściany. Obsługa myszą na płótnie:
- Kółko przybliża wokół kursora. Przeciąganie prawym lub środkowym przyciskiem przesuwa widok. Dwukrotne kliknięcie prawym przyciskiem ponownie kadruje ścianę; zaznaczenie innej ściany kadruje ją automatycznie.
- Przeciąganie lewym przyciskiem przesuwa materiał, a obrys ściany zostaje na miejscu. Pierścień obraca mapowanie, uchwyty narożne je skalują, a uchwyt środkowy ustawia punkt odniesienia tych zmian.
- Przeciąganie lewym przyciskiem z `Alt` na krawędzi kafla pochyla mapowanie, podobnie jak przeciąganie środkowym przyciskiem z `Shift`.
- Każde przeciągnięcie to jeden krok cofania, a `Esc` anuluje trwające przeciąganie.

Pod płótnem są **Przesunięcie**, **Skala**, **Kąt**, a gdy gra ich używa, także **Flagi zawartości**, **Flagi powierzchni** i **Wartość powierzchni**. Przyciski to **Wyzeruj**, **Wyrównaj do świata**, **Dopasuj**, **Odbij H**, **Odbij V** i dwa obroty o 90°. **Wyrównaj** obraca materiał wzdłuż jednej z krawędzi ściany (naciśnij ponownie, aby przejść do następnej krawędzi, z `Shift` do poprzedniej). Cztery przyciski justowania dosuwają materiał do krawędzi. **Dopasuj U** i **Dopasuj V** skalują tak, by ścianę pokrywała całkowita liczba powtórzeń; naciśnij ponownie, aby przejść do następnej liczby, i przytrzymaj `Ctrl`, aby zachować podział arkusza trim sheet.

## Klawisze UV
Gdy zaznaczone są ściany, klawisze strzałek przesuwają materiał, `[` i `]` obracają go o 15°, `,` i `.` zmniejszają lub zwiększają dwukrotnie jego skalę, `Ctrl+F` i `Ctrl+Alt+F` go odbijają, `Shift+R` go resetuje, `Alt+Shift+R` wyrównuje go do osi świata, a `Ctrl+Shift+F` dopasowuje go do każdej ściany.

**Blokada** na pasku narzędzi zachowuje mapowanie podczas przesuwania brushów, a **UV** (`U`) zachowuje je podczas ich obracania.

## Materiały narzędziowe
- skip: usuwa oznaczoną ścianę z eksportu do silnika.
- clip: tworzy kolizję bez widocznej geometrii.
- origin: oznacza punkt obrotu encji bryłowej.

Nazwy można zmienić na karcie **Materiały narzędziowe** w oknie Eksport. Używaj skip dla ścian, o których wiesz, że są niewidoczne.

## Pomijanie ścian przy eksporcie
Zaznacz ścianę i włącz **Pomiń ścianę przy eksporcie** w sekcji UV. W podglądzie UV oznacza ją pomarańczowe kreskowanie. Jej materiał i malowanie pozostają zapisane, a brush pozostaje zamknięty i edytowalny. Eksportowane siatki renderowania pomijają tę ścianę, ale kolizja zostaje zachowana. Jeśli z sześcianu zostawisz jeden czworokąt, powstaną dwa trójkąty renderowania. Nie czyni to ścian dwustronnymi ani nie scala siatek.

**Pomiń zasłonięte ściany** oznacza ściany całkowicie zasłonięte przez statyczne brushe, w tym identyczne stykające się ściany i brushe zamknięte w innych. Działa na zaznaczonych brushach, a gdy nic nie jest zaznaczone, na całej widocznej, niezablokowanej geometrii świata. Encje bryłowe, przezroczyste materiały i brushe, które już mają pominięte ściany, są pomijane. Częściowo zasłonięte ściany nie są dzielone. Jedno cofnięcie przywraca wszystko. Widok → Pokaż pominięte ściany ukrywa lub pokazuje takie ściany w widokach edytora; zaznaczanie nadal działa na całym brushu.

# Kolor i gradienty
Sekcja **Kolor** na karcie Ściana maluje ściany jednolitymi kolorami lub gradientami na wierzchu ich materiałów. Przydaje się do szkicowania poziomu (blockout) i do stylizowanej grafiki. Pomalowane ściany zachowują w widokach swoje prawdziwe kolory; zaznaczenie jest pokazywane obrysem.

## Paleta i kolor mieszany
Kliknij jedną z dwunastu próbek palety, aby pomalować nią zaznaczone ściany albo całe zaznaczone brushe. **Krycie** ustawia w procentach krycie następnego malowania. **Mieszanie** ma przycisk koloru, który otwiera próbnik kolorów; przeciągaj w próbniku, a zaznaczenie zmienia się na żywo, albo naciśnij **Malowanie**, aby nałożyć kolor mieszany w jednym kroku. **Wróć do materiału** usuwa malowanie, tak że ściana znowu pokazuje swój materiał. Wiersz na górze sekcji podaje, co jest zaznaczone i co ma na sobie.

## Próbnik kolorów
Każdy przycisk koloru w edytorze otwiera ten sam próbnik. Przeciągaj w kwadracie, aby wybrać nasycenie i jasność, a w pasku obok, aby wybrać odcień. Pod spodem są wartości **R**, **G** i **B** oraz suwak **Krycie**, jeśli kolor go ma. Pole szesnastkowe przyjmuje `#RRGGBB` lub `#RRGGBBAA`; zmienia kolor na czerwony, dopóki tekst nie jest poprawnym kolorem. **Kopiuj** umieszcza kod szesnastkowy w schowku. Kliknij poza próbnikiem, aby go zamknąć.

## Gradienty
**Gradient** wybiera Gradient liniowy lub Gradient radialny, a przycisk obok odwraca kolejność kolorów. Każdy punkt koloru ma przycisk koloru i **Pozycja (%)**, a punkty między końcami mają przycisk usuwania. **Dodaj punkt koloru** dodaje punkt w największej luce; gradient może mieć do ośmiu punktów. **Kąt gradientu** ustawia kierunek gradientu liniowego, z przyciskami **+45°** i **−45°**.

Pod podglądem jest szyna ze znacznikiem dla każdego punktu. Kliknij szynę, aby dodać punkt, kliknij znacznik, aby otworzyć jego próbnik, przeciągnij znacznik, aby go przesunąć, i kliknij znacznik prawym przyciskiem, aby go usunąć. Trzy próbki pod szyną to gotowe ustawienia: Ciepły gradient, Chłodny gradient i Gradient szarości. Zmiany gradientu są stosowane do zaznaczenia na żywo.

Kierunek gradientu ma znaczenie: ustaw go wzdłuż spadku dachu albo wzdłuż zakrzywionego obiektu, a nie w poprzek niepowiązanych ścian. Po malowaniu obejrzyj model z obu stron.

## Materiał powierzchni
**Materiał powierzchni** ustawia **Chropowatość** (0 to połysk, 100 to mat), **Metaliczność** (0 dla drewna, kamienia lub plastiku, 100 dla metalu) i **Emisja** (od 0 do 8; powierzchnia świeci własnym kolorem, także gradientami, ale nie oświetla pobliskich obiektów). Zmiany są stosowane na żywo do zaznaczonych ścian lub brushów. **Użyj domyślnej powierzchni** usuwa nadpisanie. Wszystkie zmiany można cofnąć.

Efekty powierzchni widać w oświetlonym widoku 3D i na renderach z kamery. Wyświetlanie płaskie i siatka krawędzi celowo pomijają odblaski. Bloom może dodać poświatę wokół powierzchni emisyjnych. Godot i glTF/GLB otrzymują wartości materiału, a importer Unity zamienia chropowatość na gładkość (smoothness). Ograniczenia poszczególnych celów opisuje `docs/surface-materials.md`.

# Assety, warstwy i grupy
## Biblioteka assetów
Otwórz kartę **Assety**. Folder modeli `.map` dodasz przyciskiem **Dodaj folder**; wbudowana biblioteka jest dostępna zawsze. Wpisz tekst w **Filtruj assety**, aby wyszukiwać. Kliknij model, aby go zaznaczyć, a potem kliknij go dwukrotnie, naciśnij `Enter` lub kliknij **Wstaw**, aby dodać go tam, gdzie ostatnio był kursor. Model możesz też przeciągnąć do widoku. Brushe assetu są wstawiane jako jedna grupa, więc model porusza się jako całość.

**Zapisz zaznaczenie jako asset** zapisuje zaznaczone brushe do biblioteki jako plik `.map`. Assety z biblioteki to zwykłe edytowalne brushe, a nie zaimportowane siatki.

## Warstwy
Lista **Warstwy** na karcie Mapa porządkuje duże obszary. Wpisz nazwę i kliknij **Dodaj**, aby utworzyć warstwę, albo naciśnij `Enter`, aby zmienić nazwę bieżącej. Każdą warstwę można ukryć, zablokować albo wyłączyć z eksportu. **Przenieś tutaj** przenosi zaznaczenie do bieżącej warstwy. Menu kontekstowe w widokach oferuje też Przenieś do warstwy, Ustaw jako aktywną warstwę, Ukryj warstwy i Izoluj warstwy.

## Grupy i połączone grupy
`Ctrl+G` grupuje zaznaczenie, a `Ctrl+Shift+G` je rozgrupowuje. Sekcja **Grupy** na karcie Mapa wyświetla grupy i oferuje **Otwórz**, **Zmień nazwę**, **Scal**, **Rozgrupuj**, **Wyjmij** i **Wydziel**. Połączona grupa to zestaw kopii, które współdzielą zmiany: `Ctrl+Shift+D` tworzy połączony duplikat, Geometria → Zsynchronizuj połączone grupy aktualizuje kopie, a **Rozłącz** usamodzielnia kopię. Na karcie Encja można wskazać właściwość, która przy synchronizacji połączonych grup pozostaje własną wartością danej kopii.

## Wspólna skala
Domyślna skala eksportu to 32 jednostki mapy na metr. Wbudowany znacznik startu gracza ma 64 jednostki wysokości, czyli 2 metry, i służy jako punkt odniesienia. Porównuj z nim drzwi, pojazdy, meble i rośliny. Skalę eksportu zmieniaj tylko celowo i dbaj o spójność powiązanych assetów.

# Encje i rozgrywka
Popularne definicje encji są wbudowane i zawsze dostępne: światła, punkty startu gracza, drzwi, przyciski, platformy, wyzwalacze, statyczne rekwizyty, dźwięki i punkty nawigacyjne. Wczytanie pliku FGD projektu przez **Definicje encji** na karcie Encja zastępuje klasy o tej samej nazwie, a pozostałe wbudowane zachowuje.

## Umieszczanie encji
W przeglądarce **Encje** na karcie Encja wpisz tekst w **Filtruj klasy** i wybierz klasę. Dla klasy punktowej, na przykład światła, kliknij **Umieść** lub kliknij klasę dwukrotnie; `Insert` umieszcza encję w środku zaznaczenia. Dla klasy bryłowej, na przykład drzwi lub wyzwalacza, zaznacz brushe i kliknij **Utwórz**. Klasy można też przeciągać do widoku.

## Właściwości
Lista właściwości edytuje zaznaczoną encję. Encje tej samej klasy edytuje się razem. **Dodaj właściwość** dodaje klucz, którego definicja może nie znać; **Domyślne** może przywrócić istniejącym właściwościom wartości domyślne albo dodać brakujące. Właściwości, które klasa definiuje, ale encja ich nie ma, są przyciemnione; wpisanie wartości je dodaje. Do łączenia encji używaj targetname i target.

## Co potrafi eksport
Dyrektywy Godota tworzą natywne węzły, ciała kolizyjne i obszary wyzwalaczy. Unity i Unreal obsługują natywnie popularne światła, a Unreal tworzy też aktory PlayerStart. Pozostałe dane są zachowywane w takim zakresie, na jaki pozwala importer docelowego silnika.

Sama encja drzwi nie dodaje zachowania w grze. Ruch, reakcje na wyzwalacze, kontrolery gracza i skrypty projektu nadal wymagają pracy w silniku. Defold eksportuje właściwości encji i obiekty kolizyjne; Flax umieszcza modele i ogólne encje. Sprawdź wynik w silniku, zanim zbudujesz na nim rozgrywkę.

# Animacja
Heron Brush potrafi podglądać prosty ruch i edytować animacje encji bryłowych oparte na klatkach kluczowych, a następnie eksportować je do kilku silników.

## Podgląd animacji
Włącz **Podgląd animacji** w menu Widok, menu Animacja, menu Ustawienia siatki albo w sekcji Widok na karcie Mapa. Podgląd odtwarza obrót func_rotate i ruch tam i z powrotem func_shuttle, gdy włączone jest start_on, korzystając z właściwości encji: osi, prędkości, kierunku, drogi i czasu oczekiwania. Odtwarza też klipy klatek kluczowych ustawione na Odtwarzaj przy starcie. Assety wiatraka i wentylatora już zawierają obracające się części.

Wyłączenie podglądu przywraca pierwotną pozę. Kliknięcie w widoku zatrzymuje odtwarzanie przed zaznaczaniem lub edycją; nawigacja kamerą nadal działa. Podgląd nie zmienia mapy, historii cofania ani eksportu i nie uruchamia fizyki ani logiki gry.

## Edytor klatek kluczowych
Otwórz go przez Animacja → Animacja klatkami kluczowymi albo z menu Ustawienia siatki. Dokuje się pod widokami albo w oknie 3D, gdy jest ono osobne. Ikona okna w jego nagłówku przenosi go do osobnego okna; zachowuje przy tym klatkę i historię cofania. **Edytuj animację** przełącza widoki na animowaną pozę, a **Edytuj mapę** wraca do geometrii spoczynkowej. W trybie animacji każdy widok pokazuje u dołu **Edytuj animację** i numer klatki.

Zaznacz encję bryłową albo brush lub grupę brushów, aby ją animować. Płaska grupa brushów ma wspólną animację i punkt obrotu; grupy zawierające kilka encji wymagają osobnych animacji.

## Klipy
Selektor **Animacja** przełącza między nazwanymi klipami. Wpisz nazwę w **Nazwa animacji** i kliknij **Zapisz jako**, aby skopiować bieżący klip pod tą nazwą; **Usuń** usuwa klip. **Przesunięcie** tworzy klip `translate` bez obrotu. **Animacje drzwi** tworzy płynne klipy `open` i `close` z obrotem o 90 stopni wokół bieżącego punktu obrotu; najpierw przenieś punkt obrotu na zawias. **Odtwarzaj przy starcie** sprawia, że klip odtwarza się po uruchomieniu poziomu; gotowe ustawienie dla drzwi to wyłącza.

## Sterowanie odtwarzaniem
Przyciski sterowania to **Początek**, **Poprzedni klucz**, **Odtwarzanie**, **Następny klucz** i **Koniec**. **Klatka** pokazuje i ustawia bieżącą klatkę, a **Czas trwania (s)** ustawia długość klipu. Tryb to **Raz**, **Pętla** lub **Tam i z powrotem**.

## Oś czasu
Oś czasu ma linijkę z głowicą odtwarzania oraz ścieżkę dla **Pozycja** (X, Y i Z kluczowane razem) i dla każdego kąta obrotu, RX, RY i RZ. Kliknij lub przeciągnij po linijce, aby przesunąć głowicę i podejrzeć tę pozę we wszystkich widokach. Przeciąganie środkowym przyciskiem przesuwa oś czasu. Przeciągaj klucze, aby przesunąć je w czasie; `Shift` dodaje do zaznaczenia; dwukrotne kliknięcie ścieżki wstawia klucz; `Ctrl`+kółko przybliża. **Krzywe** pokazuje krzywe wartości zamiast ścieżek.

Przyciski nad nią to **Dodaj klucz**, **Usuń**, **Kopiuj**, **Wklej** i **Duplikuj**. **Przyciągaj do klatek** utrzymuje klucze na pełnych klatkach, a **Dopasuj** pokazuje cały klip. **Zmień czas kluczy** otwiera **Skala czasu**, **Przesunięcie (klatki)**, **Skaluj od kursora**, **Wydłuż animację** i **Zmień czas zaznaczenia**.

Inspektor kluczy obok osi czasu edytuje zaznaczony klucz: **Klatka**, **Wartość** (albo trzy współrzędne klucza Pozycja), interpolację **Liniowa**, **Stała** lub **Béziera**, **Nachylenie wejściowe** i **Nachylenie wyjściowe** oraz **Wygładź styczne**.

## Klawisze w edytorze
Gdy oś czasu ma klawiaturę: `Space` odtwarza lub wstrzymuje, strzałki w lewo i w prawo przechodzą o jedną klatkę, `Ctrl+A` zaznacza wszystkie klucze, `Ctrl+C`, `Ctrl+X` i `Ctrl+V` kopiują, wycinają i wklejają klucze, `Ctrl+D` je duplikuje, `Delete` je usuwa, `Ctrl+Z` cofa, a `Esc` anuluje przeciąganie. Poza osią czasu `Esc` opuszcza tryb animacji. Cofanie pozostawia przybliżenie, zaznaczenie i głowicę odtwarzania tam, gdzie były.

## Pozowanie w widokach
Wiersz pod osią czasu zawiera **Podgląd sąsiednich klatek** z liczbą i **Odstęp**, co pokazuje pobliskie pozy jako duchy; współrzędne **Punkt obrotu**; **Automatyczne klucze**; oraz narzędzia pozy **Przesunięcie**, **Obróć** i **Punkt obrotu**.

- **Dodaj klucz** włącza Automatyczne klucze i wstrzymuje odtwarzanie. Gdy Automatyczne klucze są włączone, a wybrane jest **Przesunięcie**, przeciągnij kolorową oś uchwytu zaznaczonej encji w widoku 3D, aby zakluczować jej pozycję w bieżącej klatce.
- W widoku 2D przeciągnij zaznaczony brush lub element jego grupy, aby zakluczować przesunięcie wzdłuż dwóch widocznych osi, z przyciąganiem do siatki. Automatyczne klucze nie muszą być do tego włączone.
- Z narzędziem **Obróć** przeciągnij brush w widoku 2D wokół jego pomarańczowego znacznika punktu obrotu. Widok Z góry kluczuje RZ, Z przodu RY, a Z boku RX, z przyciąganiem do kątów siatki.
- Z narzędziem **Punkt obrotu** przeciągnij żółty środek obrotu w dowolnym widoku 2D. Przyciąga się on do siatki i zachowuje swoją głębokość. Klucze i geometria spoczynkowa pozostają bez zmian.

Turkusowa ścieżka w 2D i 3D pokazuje drogę punktu obrotu. Kliknij punkt ścieżki, aby wybrać jego czas i klucze, albo przeciągnij go, aby zmienić pozycję; przy włączonych Automatycznych kluczach możesz przeciągać punkty ścieżki także w widoku 3D. Etykiety pokazują numery klatek, a przerywane odcinki oznaczają skoki. `Esc` anuluje każde z tych przeciągnięć, a jedno cofnięcie odwraca całe przeciągnięcie.

Nowe animacje zaczynają się w spoczynku, z kluczami w czasie zero. Ruch tworzysz, zmieniając pozę w innej klatce. Aby uzyskać ostrą ścieżkę w kształcie litery L, zakluczuj początek, narożnik i koniec, a następnie użyj interpolacji Liniowa z trybem Tam i z powrotem.

## Łączenie przycisków z animacjami
Zaznacz func_button, trigger_multiple lub trigger_once. Karta Encja pokazuje nad właściwościami **Wyzwalacz animacji**. Wybierz cel z listy albo kliknij **Wybierz cel w widoku**, a potem animowany obiekt w dowolnym widoku; `Esc` anuluje wybieranie. Brakujące cele lub klipy są zgłaszane pod kontrolkami. Gdy wyświetlanie połączeń jest włączone, połączenia encji pokazują strzałki do swoich celów.

**Akcja** wybiera, co robi aktywacja: **Odtwórz** uruchamia nazwany klip lub wybraną animację; **Przełącz** odtwarza na przemian `open` i `close`, jeśli istnieją, a w przeciwnym razie uruchamia wybraną animację od nowa; **Otwórz** i **Zamknij** uruchamiają te klipy; **Wstrzymaj** zatrzymuje pozę, **Wznów** kontynuuje, a **Zatrzymaj** przywraca pierwotną pozę. **Tylko raz** nadpisuje domyślne ustawienie klasy. **Odstęp powtórzeń (s)** ustawia najkrótszy czas między aktywacjami; nie opóźnia pierwszej.

**Wypróbuj wyzwalacz** wykonuje akcję w podglądzie bez edytowania mapy i bez dodawania kroków cofania. **Zresetuj podgląd** przywraca pierwotne pozy i czyści stan jednorazowości i odstępów. Nie symuluje to kolizji gracza ani skryptów gry.

## Animacja w eksporcie
Godot eksportuje natywne zasoby AnimationPlayer i AnimationLibrary, łącznie z krzywymi i ruchomą kolizją. CharacterBody3D wchodzący w obszar przycisku aktywuje jego cel. Zmiany wprowadzone w tych klipach w Godocie nie wracają do mapy i zostaną zastąpione przy następnym eksporcie.

Eksport do raylib dodaje `<map>_animation.h` z odtwarzaczem w C i `<map>_preview.c` z podglądem ze swobodną kamerą; aby go zbudować, postępuj zgodnie z `<map>_raylib.md`. W podglądzie `SPACE` wstrzymuje, `R` uruchamia od nowa domyślne klipy, a `N` przełącza kolejne nazwane klipy. Defold wbudowuje odtwarzanie w skrypt encji z udokumentowanymi komunikatami `heron_*_animation`. Build Level w Unity dodaje `HeronBrushAnimation` i ciało kinematyczne. Eksport do Bevy zawiera `heron_animation.rs` i instrukcje. Godot, Unity, Defold, Bevy i raylib zachowują połączenia wyzwalaczy, flagę jednorazowości i odstęp powtórzeń. Twoja gra nadal musi zarejestrować odtwarzacz i połączyć własne przyciski z API danego środowiska uruchomieniowego.

## Przykład Motion lab
Otwórz `motion_lab` z sekcji Przykłady na ekranie powitalnym. Zawiera platformę poruszającą się po ścieżce w kształcie L, windę, obracającą się platformę i drzwi na zawiasach z przyciskami. Otwórz w Godocie leżący obok plik `godot/project.godot` i naciśnij `F5`. WASD i mysz poruszają graczem, Spacja to skok, 1 do 4 przenoszą do kolejnych stanowisk, a R uruchamia bieżące stanowisko od nowa. Wejdź na kolorową płytkę, aby aktywować jej cel. Dołączony plik README wyjaśnia, jak eksportować ponownie.

# Eksport do silnika
Plik → Eksport otwiera okno Eksport. Silniki są wymienione po lewej wraz z logo: Godot, Defold, three.js, Bevy, raylib, Unity, Unreal Engine, Flax Engine, Cocos Creator, Wavefront .obj, glTF .gltf, GLB .glb i na końcu `.map`. Strona po prawej opisuje, co otrzyma wybrany cel. **Zapisz** zapisuje eksport, **Sprawdź** buduje poziom bez zapisywania i wyświetla problemy, a **Zamknij** zamyka okno.

Eksport jest niezależny od zapisywania: plik `.map` pozostaje twoim edytowalnym źródłem. Każda kontrolka zapisuje ustawienia swojego silnika od razu po zmianie, więc każdy silnik pamięta własne odpowiedzi, a okno otwiera się tak, jak je zostawiłeś.

## Wspólne opcje
- **Skala**: **Jednostki na metr**, domyślnie 32.
- **Co zapisać**: **Geometria kolizji**.
- **Optymalizacje**: **Grupuj statyczne siatki w komórkach po 1024 jednostki**, **Odrzuć ściany zagrzebane wewnątrz geometrii**, **Połącz ściany leżące w jednej płaszczyźnie** i **Połącz kształty kolizji**.
- **Ścieżki projektu**: gdzie w projekcie trafia poziom i jego zasoby.
- **Materiały narzędziowe**: nazwy materiałów clip, skip i origin.
- **Więcej**: przełączniki specyficzne dla silnika, takie jak **Nazwij węzły według**, **Odtwórz warstwy i grupy jako węzły nadrzędne**, **Odwróć kolejność wierzchołków trójkątów**, odwracanie współrzędnych tekstur oraz drugi zestaw UV dla wypalonego światła z ustawieniem **Teksel mapy światła**.
- **Aktualizuj wraz z mapą**: **Zapisuj ponownie przy każdym zapisie mapy**. Opcja staje się dostępna po pierwszym eksporcie do danego silnika, bo to on określa, gdzie zapisywać.

Komórki statyczne przydzielają brushe świata według ich środków i łączą je według materiału w obrębie każdej komórki o boku 1024 jednostek, dzięki czemu silniki mogą odrzucać mniejsze części mapy. Kolizja zostaje zachowana, a encje rozgrywki pozostają osobne. Może to oznaczać więcej wywołań rysowania niż jedna siatka świata, dlatego domyślnie opcja jest wyłączona. Samodzielne eksporty OBJ, glTF i GLB nie używają komórek.

## Uwagi o celach eksportu
- Godot: ścieżki `res://` tekstur i materiałów, warstwa fizyki wyzwalaczy i UV2 dla lightmap. Wybierz folder eksportu wewnątrz projektu Godot.
- Defold: katalog zasobów assetów, materiał modelu i sampler tekstury, który musi istnieć w wybranym materiale.
- Unity: siatki OBJ i importer edytora; eksportuj do folderu Assets projektu.
- Unreal Engine: wygenerowany skrypt importu i ścieżka zawartości, zwykle /Game/<level>.
- Flax Engine: ścieżka zawartości modelu względem folderu Content projektu.
- three.js, Bevy, raylib i Cocos Creator: sceny lub kod dla tych środowisk, z instrukcjami obok eksportu.
- glTF / GLB: przenośna geometria dla zgodnych narzędzi; GLB to postać binarna. OBJ: geometria i materiały dla programów do modelowania.
- `.map`: **Zapisz jako** wybiera Valve 220, Standard, Quake 2 lub Quake 2 (Valve). Okno informuje, co wybrany dialekt by utracił.

Po zapisaniu przeczytaj ostrzeżenia, na przykład o brakujących teksturach. **Wyniki kontroli** wyświetla je w oknie.

# Połączenia na żywo: Godot, Unity, Defold i Flax
Połączenie na żywo przesyła niezapisane zmiany z Heron Brush do edytora silnika na bieżąco w trakcie pracy. Działa w jedną stronę: zmiany wprowadzone w silniku nie wracają. W jednym oknie mapy może działać jedno połączenie na żywo. Zatrzymaj je przed zmianą silnika i uruchom ponownie, aby zastosować zmienione opcje eksportu. Kontrolki znajdują się w oknie Eksport, na stronie silnika, na karcie **Połączenie na żywo z**.

Wskaźnik na pasku tytułu pokazuje stan połączenia; kliknij go, aby zatrzymać połączenie. Zamknięcie lub zastąpienie mapy zatrzymuje publikowanie. Po nieudanym imporcie usuń problem i uruchom połączenie ponownie.

## Godot
1. Wyeksportuj mapę raz do projektu Godot.
2. Kliknij **Zainstaluj dodatek Godot**.
3. Włącz Heron Brush Live w Godocie w Project Settings → Plugins.
4. Kliknij **Uruchom połączenie na żywo** i dołącz mapę z panelu Godota do otwartej sceny 3D.

Dodatek zastępuje tylko wygenerowaną przez siebie gałąź i zachowuje położenie dołączenia oraz inne węzły. Scenę Godota zapisuj sam, a własne skrypty i węzły trzymaj poza wygenerowaną gałęzią.

## Unity
1. Wyeksportuj mapę raz do folderu Assets projektu Unity.
2. Kliknij **Zainstaluj dodatek Unity** i pozwól Unity go skompilować.
3. Otwórz w Unity Window → Heron Brush → Live Link.
4. Uruchom połączenie i dołącz mapę do aktywnej sceny Unity.

Okno połączenia na żywo w Unity musi pozostać otwarte. Aktualizacje są wstrzymywane w Play Mode i Prefab Mode. Zastępowany jest tylko element podrzędny Generated obiektu mapy. Nie edytuj wygenerowanych plików.

## Defold
1. Wyeksportuj mapę raz do projektu Defold i pozostaw projekt otwarty w Defoldzie.
2. Kliknij **Uruchom połączenie na żywo**. Żaden dodatek nie jest potrzebny.
3. Otwórz w Defoldzie heron_brush_live/preview.collection albo odwołaj się do niej z własnej sceny.

Połączenie zapisuje do osobnego wygenerowanego folderu i nie zmienia końcowego eksportu ani pliku game.project. **Odświeżaj uruchomioną grę** aktualizuje także grę uruchomioną z Defolda; dodanie obiektów lub zmiana kolizji może wymagać ponownego uruchomienia poziomu. Nieudane połączenia są ponawiane automatycznie.

## Flax
Wyeksportuj mapę do folderu Content projektu i kliknij **Zainstaluj dodatek Flax**. Pozwól Flaxowi go skompilować, uruchom połączenie, a następnie w otwartej scenie Flax wybierz Tools → Heron Brush: attach live map. Wygenerowany korzeń zachowuje swoje położenie, a inne obiekty pozostają nienaruszone. Scenę zapisuj sam. Aktualizacje są wstrzymywane w Play Mode. Tools → Heron Brush: stop live map wstrzymuje odbiór.

# Wspólne sesje na żywo
Współpraca → Sesja na żywo otwiera okno do prowadzenia sesji albo dołączania do sesji innego użytkownika Heron Brush. Pozwala to wspólnie edytować mapę z innymi osobami; to coś innego niż połączenie na żywo z silnikiem.

## Prowadzenie i dołączanie
Wpisz **Twoja nazwa**. Aby poprowadzić sesję, wybierz **Utwórz sesję**, kliknij **Uruchom sesję** i wyślij gościom dane z **Kopiuj dane połączenia**. Aby dołączyć, wybierz **Dołącz do sesji**, wklej dane i kliknij **Dołącz**. Dołączenie zastępuje otwartą mapę mapą gospodarza, więc najpierw zapisz swoją pracę.

Standardowa kompilacja działa w sieci lokalnej, a **Port sieci lokalnej** ustawia port. Sesje przez internet (**Połącz przez internet**) wymagają kompilacji z opcjonalną funkcją sieciową. Uczestnicy widzą w widoku 3D nawzajem swoje znaczniki kamer i nazwy.

Zamknięcie okna nie kończy sesji; jej stan pokazuje wskaźnik na pasku tytułu. **Zatrzymaj sesję** lub **Opuść sesję** ją kończy. Gdy dwie osoby edytują ten sam obiekt, o ostatecznej wersji decyduje kolejność gospodarza, a cofnięcie też jest wspólną edycją. W miarę możliwości pracujcie w osobnych obszarach i zapisujcie punkty kontrolne.

# Wyświetlanie 3D i drugi monitor
## Tryby wyświetlania
Sekcja **Widok** na karcie Mapa przełącza, co rysują widoki: Brushe, Encje punktowe, Modele encji, Nazwy klas encji, Obrysy grup, Patche, Materiały, Pokaż pominięte ściany, Podgląd animacji, Siatka krawędzi, Krawędzie i Obrysy encji bryłowych. Siatka krawędzi rysuje krawędzie trójkątów, także przekątne i krawędzie za powierzchniami, bez materiałów. To tymczasowy widok do debugowania, który nie wpływa na eksport ani rendery.

Oświetlenie, cienie, okluzja otoczenia (ambient occlusion), odbicia, bloom i wygładzanie widoku 3D znajdziesz w Preferencje → Renderer. To ustawienia podglądu; końcowe obrazy używają własnych ustawień kamery renderowania.

## Osobne okno 3D
Widok → Widok 3D w osobnym oknie przenosi widok 3D do osobnego okna, które możesz umieścić na innym monitorze. Współdzieli kamerę, dokument i narzędzia, a płaskie widoki zajmują jego miejsce w głównym oknie. Jego menu oferuje Materiały, Widok bryłowy i Siatka krawędzi; podmenu Widok z pozycjami Pokaż pominięte ściany, Podgląd animacji, Krawędzie, Obrysy grup, Encje punktowe, Modele encji i Obrysy encji bryłowych; Ustaw ostrość na zaznaczeniu; Z przodu, Z boku i Z góry, które obracają kamerę wokół zaznaczenia (lub punktu przed kamerą); Umieść kamerę według widoku 3D; Pełny ekran (`F11`); Renderuj obraz; oraz Wróć do głównego okna. Zamknięcie okna z powrotem dokuje widok.

## Niebo proceduralne
Niebo proceduralne jest w widoku 3D domyślnie włączone i można je wyłączyć w Preferencje → Renderer → Światło. Kamery renderowania mają własny przełącznik **Niebo proceduralne**, zapisywany z mapą. Kolory nieba, horyzontu i ziemi pochodzą z ustawień otoczenia, a słońce podąża za oświetleniem sceny. Niebo nie ma geometrii i nie jest eksportowane.

# Kamery renderowania i obrazy PNG
Plik → Renderuj obraz otwiera okno renderowania: kamera i jej ustawienia po lewej, a po prawej podgląd na żywo, który odświeża się przy każdej zmianie. Kamery są zapisywane w pliku `.map` w warstwie Render cameras, dostępnej tylko w edytorze i pomijanej przy eksporcie do silników.

## Ustawianie kamery
Skadruj ujęcie w widoku 3D i naciśnij `Ctrl+Alt+0` (lub `Ctrl+Alt+Num0`) albo kliknij **Umieść kamerę według widoku 3D**. Jeśli żadna kamera nie jest zaznaczona, aktualizowana jest pierwsza zapisana; jeśli nie ma żadnej, zostaje utworzona nowa. Można to cofnąć, a ustawienia wyjścia i oświetlenia pozostają bez zmian. **Wybierz kamerę na mapie**, **Wczytaj kamerę z mapy** i **Przenieś widok 3D do kamery** działają w przeciwnym kierunku. **Kadr kamery w widoku 3D** pokazuje granice obrazu wyjściowego, a **Steruj kamerą z widoku 3D** pozwala zwykłej nawigacji poruszać zapisaną kamerą.

## Ustawienia
- **Pozycja kamery**: X, Y, Z, odchylenie (yaw) i pochylenie (pitch).
- **Wyjście i obiektyw**: **Ustawienia wstępne** (Full HD, Kwadrat, Pionowo), szerokość, wysokość, liczba próbek, **Pionowe pole widzenia** i **Daleka płaszczyzna odcięcia**.
- **Oświetlenie**: Światło dzienne, Zachód słońca lub Pochmurno albo własna ekspozycja, słońce, kolory nieba, horyzontu i ziemi, **Niebo proceduralne**, **Cienie**, **Ambient occlusion** i **Bloom**.
- **Głębia ostrości**: **Włącz głębię ostrości**, odległość ostrości, zakres ostrości i promień rozmycia. **Ustaw ostrość na zaznaczeniu** ustawia ostrość na środku zaznaczonego obiektu przed kamerą.

**Zapisz kamerę i ustawienia** zapisuje formularz w mapie; zapisz mapę, aby zachować go na dysku.

## Renderowanie
**Renderuj PNG** pyta, gdzie zapisać plik, i rozpoczyna renderowanie progresywne, które w podglądzie wypełnia się próbka po próbce. Więcej próbek wygląda lepiej, ale trwa dłużej. **Anuluj renderowanie** przerywa bez zapisywania pliku. Renderer korzysta z rastrowego potoku edytora, a nie ze śledzenia ścieżek (path tracing). Plik PNG ma nieprzezroczyste tło, a animacje nie są renderowane. Bardzo duże obrazy mogą na chwilę wstrzymać edytor.

# Preferencje
Plik → Preferencje (`Ctrl+,`) otwiera ustawienia. Zmiany działają od razu i są zapisywane. Pasek boczny zawiera strony Przegląd, Gry, Widok, Kolory, Mysz, Klawiatura i Renderer. Pole wyszukiwania u góry znajduje ustawienie na dowolnej stronie po nazwie lub opisie. Każde zmienione ustawienie ma własny przycisk przywracający wartość domyślną, a **Przywróć domyślne** resetuje bieżącą stronę.

## Strony
- **Przegląd**: najczęstsze ustawienia w jednym miejscu: Wygląd (Krycie paneli (%), Schemat kolorów, Układ, Język), Edycja, Widok i Podręcznik.
- **Gry**: gra, położenie jej plików i twoja nazwa do współpracy. Gra określa, w jakim formacie zapisywać mapę, które definicje encji wczytać i gdzie są jej materiały. Konfiguracje gier są czytane z folderu `games` w folderze konfiguracji (`~/.config/heron-brush/games` na Linuksie i macOS, `%LOCALAPPDATA%\heron-brush\games` na Windows) oraz z folderu `games` obok aplikacji, do którego pakiety wkładają dołączoną konfigurację.
- **Widok**: Układ, Schemat kolorów, Język, osie świata, krycie siatki, rozmiar komórki przy sprawdzaniu nieszczelności i blokady mapowania.
- **Kolory**: kolory widoków, geometrii i uchwytów.
- **Mysz**: prędkości i odwrócenia rozglądania się, przesuwania widoku i ruchu, prędkość lotu, pole widzenia, Powiąż widoki 2D i rozmiary uchwytów.
- **Klawiatura**: układ skrótów, Heron Brush lub Blender.
- **Renderer**: realistyczne cieniowanie, obrys zaznaczenia, światło, cienie, okluzja otoczenia, odbicia, bloom i powierzchnie.

## Język
**Język** oferuje Zgodnie z systemem i każdy przetłumaczony język. Interfejs zmienia się natychmiast. Podręcznik otwiera się w języku interfejsu, jeśli istnieje jego tłumaczenie, a w przeciwnym razie po angielsku.

# Wtyczki i MCP
## Wtyczki
Wtyczki rozszerzają edytor o skrypty Rhai. Plik → Wtyczki otwiera okno Wtyczki. **Zainstaluj wtyczkę** instaluje plik `.rhai`, a **Zainstaluj folder wtyczki** instaluje folder zawierający `main.rhai`; opcjonalne metadane `plugin.toml` opisują wtyczkę i jej ustawienia. Wtyczki są pogrupowane w **Aktywne wtyczki** i **Nieaktywne wtyczki**, a **Włącz wtyczkę** włącza lub wyłącza wtyczkę; ustawienie przetrwa ponowne uruchomienie. **Wczytaj wtyczki ponownie** odczytuje skrypty z dysku jeszcze raz i resetuje wartości w ich panelach.

Aktywne wtyczki pojawiają się też w sekcji **Wtyczki** na karcie Mapa ze swoimi ustawieniami, przyciskami akcji (**Uruchom**) i wynikami. Akcja działa na zaznaczeniu, a gdy nic nie jest zaznaczone, na wszystkim, i stanowi jeden krok cofania. Dołączona wtyczka Selection inspector pokazuje liczby elementów oraz akcje przesunięcia w górę i w dół.

## MCP
Współpraca → Udostępnij agentowi (MCP) pozwala agentowi AI lub innemu narzędziu sterować otwartym edytorem przez Model Context Protocol. Okno pokazuje adres lokalny z przyciskiem **Kopiuj**, a **Zatrzymaj serwer** go wyłącza; dopóki serwer działa, pasek tytułu pokazuje **MCP aktywny**. Opcja wiersza poleceń `--mcp` uruchamia ten sam interfejs bez okna. Klienci mogą odkrywać narzędzia do geometrii, assetów, właściwości, animacji, eksportu i sesji. MCP i wtyczki edytora to różne interfejsy.

# Wiersz poleceń
Program nazywa się `heron-brush`. Bez opcji otwiera ekran powitalny.

## Otwieranie plików
`heron-brush map.map [entities.fgd] [materials/]` otwiera mapę, opcjonalnie z definicjami encji i folderem materiałów. Pusty argument pomija jeden z nich, na przykład `heron-brush map.map "" materials/`. Jeśli edytor już działa, mapa otwiera się w nim; `--new-instance` otwiera zamiast tego osobne okno.

## Polecenia
- `--help` lub `-h`: wypisz sposób użycia. `--version` lub `-V`: wypisz wersję.
- `--mcp`: udostępnij MCP przez standardowe wejście i wyjście, bez okna.
- `--merge-map <base> <ours> <theirs> [<output>]`: scalanie trójstronne plików `.map` dla gita. Domyślnym wyjściem jest ours. Konflikty są wypisywane i dają niezerowy kod wyjścia.
- `--obj-to-map <in.obj> <out.map> [size] [thickness] [snap] [material]`: zamień siatkę na brushe. Size to długość najdłuższego boku w jednostkach mapy (domyślnie 256), snap to siatka dla narożników (0 oznacza brak), a słowa `solid` lub `layered` budują zamiast cienkich klinów prostopadłościany na siatce lub wypukłe płyty.
- `--render-preview <in.map> <out.png> [size]`: zapisz miniaturę mapy w PNG, domyślnie 512 pikseli.
- `--run-script <script.rhai> <in.map> [out.map] [key=value ...]`: uruchom skrypt wtyczki na mapie z zaznaczonym wszystkim i zapisz wynik (domyślnie z powrotem do pliku wejściowego). `key=value` podaje wartości ustawień wtyczki.

# Rozwiązywanie problemów
## Gdy coś wygląda źle
- Brakujący materiał: sprawdź folder lub pakiet materiałów oraz ścieżki projektu w eksporcie.
- Zły rozmiar modelu: porównaj go z 64-jednostkowym znacznikiem gracza i skalą eksportu.
- Zły kierunek gradientu: sprawdź kąt gradientu i orientację ściany.
- Pusta scena w silniku: przeczytaj ostrzeżenia eksportu i postępuj zgodnie z instrukcjami importu dla danego celu.
- Połączenie na żywo czeka: sprawdź, czy dodatek jest zainstalowany, scena dołączona, a aktualizacje nie są wstrzymane.
- Same linie, bez ścian: wyłącz Siatka krawędzi w sekcji Widok na karcie Mapa albo wybierz Materiały lub Widok bryłowy w menu okna 3D.
- Obiekty zniknęły z widoków: `Ctrl+Shift+I` pokazuje ukryte obiekty; sprawdź warstwy i filtry w sekcji Widok.
- Klawisz nic nie robi: najpierw kliknij w widok, aby miał klawiaturę, i sprawdź układ skrótów w Preferencje → Klawiatura.
- Edycja się nie powiodła: cofnij, uprość geometrię i użyj **Sprawdź mapę** oraz **Sprawdź, czy są leaki** w sekcji **Problemy** na karcie Mapa.
- Mapa nie otwiera się z menedżera plików: mogła zostać przekazana do już działającego edytora; poszukaj tego okna.

## Ustawienia i kopie zapasowe
Preferencje, ostatnie mapy, gwiazdki, ustawienia eksportu i wtyczki znajdują się w folderze konfiguracji: `~/.config/heron-brush` na Linuksie i macOS oraz `%LOCALAPPDATA%\heron-brush` na Windows. Kopie zapasowe autozapisu są w folderze `autosave` obok każdej mapy.

## Zgłaszanie błędu
Podaj wersję edytora (Pomoc → O programie), system operacyjny, małą mapę, na której widać problem, i dokładne kroki. Panel komunikatów (`Ctrl+4`) często mówi, co poszło nie tak. Przed eksperymentami zapisz kopię.

## Jak korzystać z tego podręcznika
Wybierz rozdział w spisie treści po lewej. `Ctrl+F` pokazuje spis treści i przenosi do pola wyszukiwania; `Esc` czyści wyszukiwanie albo, gdy jest ono puste, zamyka podręcznik. **Spis treści** ukrywa lub pokazuje listę rozdziałów, a **Rozmiar czcionki** zmienia wielkość tekstu; oba ustawienia są zapamiętywane. Lista języków u góry przełącza między dostępnymi tłumaczeniami.
