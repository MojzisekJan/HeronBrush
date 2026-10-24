# Erste Schritte
Heron Brush ist ein Brush-basierter Level-Editor. Sie bauen Levels aus konvexen Brushes, bemalen und texturieren ihre Flächen, platzieren Entitäten, animieren Türen und Plattformen und exportieren das Ergebnis in eine Game-Engine. Dazu kommen eingebaute Assets, parametrische Formen, Live-Verbindungen zu Engine-Editoren, gemeinsame Bearbeitungssitzungen und das Rendern von Standbildern. Dieses Handbuch wird mit der Anwendung ausgeliefert und funktioniert auch offline.

Unter macOS funktioniert `Cmd` überall dort, wo dieses Handbuch `Ctrl` nennt, und Wahl (Option) entspricht Alt.

## Der Startbildschirm
Wenn Heron Brush ohne Map startet, öffnet sich der Startbildschirm. Links finden Sie **… fortsetzen** (setzt die zuletzt geöffnete Map fort), **Neue Map** und **Durchsuchen** zum Öffnen einer vorhandenen `.map`, außerdem **Spiel**, **Handbuch** und **Einstellungen**. Rechts steht eine Liste der zuletzt verwendeten Maps mit Vorschaubildern, darunter **Beispiele**. Der Stern (**Oben anheften**) heftet eine Map oben an die Liste an; der Papierkorb (**In den Papierkorb verschieben**) verschiebt die Datei in den Papierkorb des Systems, sodass sie wiederhergestellt werden kann. Mit den Pfeiltasten bewegen Sie sich über den Bildschirm, Enter öffnet das markierte Element. Über Datei → Willkommen kehren Sie jederzeit zum Startbildschirm zurück.

## Ihre erste Map
1. Wählen Sie **Neue Map** auf dem Startbildschirm oder Datei → Neue Map (`Ctrl+N`).
2. Wählen Sie in der Werkzeugleiste das Brush-Werkzeug (`B`) und ziehen Sie in einer 2D-Ansicht, um die ausgewählte Form zu zeichnen. Prüfen Sie ihre Tiefe in einer anderen 2D-Ansicht.
3. Wählen Sie den Brush aus und verschieben Sie ihn durch Ziehen. Über den Tab **Fläche** geben Sie ihm ein Material oder eine Farbe.
4. Speichern Sie die Quell-Map mit `Ctrl+S`. Wenn Sie sie in einem Spiel verwenden möchten, öffnen Sie Datei → Exportieren und schreiben Sie eine Szene für Ihre Engine.

## Dateien und Speichern
Die `.map`-Datei ist Ihre bearbeitbare Quelle. Speichern Sie regelmäßig: Export, Live-Verbindungen und Rendern speichern die Map nie für Sie. Datei → Speichern unter (`Ctrl+Shift+S`) ergänzt die Endung `.map`, falls Sie sie weglassen. Datei → Auf die gespeicherte Map zurückgehen verwirft ungespeicherte Änderungen und lädt die Datei neu. Wenn Sie ein Fenster schließen oder eine andere Map öffnen, während es ungespeicherte Änderungen gibt, fragt Heron Brush, ob Sie **Speichern**, **Verwerfen** oder **Abbrechen** möchten.

Solange eine gespeicherte Map ungespeicherte Änderungen hat, werden alle zehn Minuten nummerierte Sicherungen in einen Ordner `autosave` neben der Map geschrieben. Die 50 neuesten Sicherungen bleiben erhalten.

Wenn Sie eine Map öffnen, während Heron Brush bereits läuft, wird sie an das laufende Fenster übergeben, das dann in den Vordergrund kommt. Verwenden Sie `--new-instance` auf der Kommandozeile, wenn Sie ein separates Fenster möchten (siehe Kommandozeile).

# Die Benutzeroberfläche
Das Hauptfenster besteht aus einer Titelleiste mit den Menüs, der Werkzeugleiste darunter, den Map-Ansichten, dem Inspektor rechts und einer Statusleiste am unteren Rand.

## Titelleiste und Menüs
Die Titelleiste zeigt das Logo, die Menüs, in der Mitte den Namen der Map und die Fensterschaltflächen. Unter macOS sitzen die gewohnten roten, gelben und grünen Knöpfe links. Ziehen Sie an der Titelleiste, um das Fenster zu verschieben, und doppelklicken Sie darauf, um es zu maximieren. Solange etwas im Hintergrund läuft, erscheint gegen Ende der Leiste eine Anzeige: eine Live-Verbindung zu einer Engine, eine Live-Sitzung oder **MCP aktiv**. Ein Klick auf eine Anzeige beendet die Live-Verbindung oder öffnet das zugehörige Fenster.

- **Datei**: Willkommen, Neue Map (`Ctrl+N`), Map öffnen (`Ctrl+O`), Speichern (`Ctrl+S`), Speichern unter (`Ctrl+Shift+S`), Auf die gespeicherte Map zurückgehen, Exportieren, Bild rendern, Plugins und Einstellungen (`Ctrl+,`).
- **Bearbeiten**: Rückgängig (`Ctrl+Z`), Wiederholen (`Ctrl+Shift+Z`), Ausschneiden, Kopieren, Einfügen, Duplizieren (`Ctrl+D`), Löschen, Alle auswählen (`Ctrl+A`), Letzte Befehle wiederholen (`Ctrl+R`) und Wiederholbare Befehle vergessen.
- **Ansicht**: 3D-Ansicht in separatem Fenster, Gehe zu, Am Raster einrasten (`Alt+0`), Raster anzeigen (`0`), Animationsvorschau und Ausgelassene Flächen anzeigen.
- **Geometrie**: 3D-Text einfügen, Ecke verbinden, Flächen in Patches umwandeln, Auswahl aus ihrer Entität lösen und Verknüpfte Gruppen abgleichen.
- **Animation**: Schlüsselbildanimation, Animationsvorschau und Kamera aus der 3D-Ansicht platzieren (`Ctrl+Alt+0`).
- **Map**: Eine Punktdatei laden, Punktdatei neu laden und Punktdatei schließen; Eine Portaldatei laden, Portaldatei neu laden und Portaldatei schließen. Damit werden Leck-Spuren und Portale angezeigt, die Map-Compiler schreiben.
- **Zusammenarbeit**: Live-Sitzung und Für einen Agenten bereitstellen (MCP).
- **Hilfe**: Handbuch und Über.

## Die Werkzeugleiste
Die Werkzeugleiste enthält alle Werkzeuge. Die erste Gruppe brauchen Sie ständig: Auswählen, Brush, Schneiden, Eckpunkt, Kante, Fläche, Drehen und Skalieren. Die zweite Gruppe enthält Scheren, Zusammenbauen, Ziehen, Kontrollpunkt, Verrundung und Streichen. Fahren Sie mit der Maus über eine Schaltfläche, um ihren Namen und ihr Tastenkürzel zu sehen. Ein erneuter Klick auf das aktive Werkzeug kehrt zu Auswählen zurück.

Nach den Werkzeugen folgen **Waagerecht spiegeln** und **Senkrecht spiegeln**, dann zwei Schalter: **Sperre** behält die Materialzuordnung bei, wenn ein Brush verschoben wird, und **UV** behält sie bei, wenn ein Brush gedreht wird (`U` schaltet um). Danach kommen die Einstellungen des aktiven Werkzeugs: Das Brush-Werkzeug zeigt die Form und ihre Optionen, das Werkzeug Verrundung zeigt Radius, Flächen und Ecken mit **Anwenden**. Am rechten Ende stehen der Werkzeugname, die Rastergröße (**Raster 16** usw.; ein Klick wählt eine andere) und das Menü **Rastereinstellungen** mit Am Raster einrasten, Raster anzeigen, Animationsvorschau, Ausgelassene Flächen anzeigen und Schlüsselbildanimation.

## Map-Ansichten
Es gibt vier Ansichten: die 3D-Ansicht und drei flache Ansichten, Oben (XY), Vorne (XZ) und Seite (YZ). Einstellungen → Ansicht → Aufteilung wählt zwischen Eine Ansicht, Zwei Ansichten, Drei Ansichten und Vier Ansichten. Ziehen Sie an den Trennlinien, um die Bereiche in der Größe zu ändern. Ein Klick in eine Ansicht gibt ihr den Tastaturfokus. `Space` schaltet einen Bereich, der jeweils nur eine Ansicht zeigt, zur nächsten Ansicht weiter, und `Ctrl+Space` maximiert die Ansicht mit dem Tastaturfokus und stellt die Aufteilung wieder her.

Ein Rechtsklick ohne Ziehen öffnet in jeder Ansicht ein Kontextmenü für die Auswahl und das Objekt unter dem Mauszeiger. Es bietet Gruppieren, Ebenen, verknüpfte Gruppen, das Erstellen von Entitäten, das Verschieben von Brushes in eine Entität hinein oder aus ihr heraus, das Kopieren eines Materialnamens und das Anzeigen des Materials im Materialbrowser.

## Der Inspektor
Der Inspektor auf der rechten Seite hat vier Tabs. Ziehen Sie an seinem linken Rand, um seine Breite zu ändern.

- **Map** (`Ctrl+1`): Ebenen, Gruppen, Plugins, Probleme, Ansichtsfilter und Marken.
- **Entity** (`Ctrl+2`): der Entitätsbrowser, die Entity-Definitionen, die Eigenschaften der ausgewählten Entität und, bei Schaltern und Triggern, Animationsauslöser.
- **Fläche** (`Ctrl+3`): UV, Materialien, Farbe, Ziehen, Verrundung, Patch und Einzug & Abschrägung. Jeder Abschnitt lässt sich einklappen.
- **Assets**: die Bibliothek wiederverwendbarer Brush-Modelle.

`Ctrl+5` blendet den Inspektor aus oder ein, `Ctrl+Alt+T` die Werkzeugleiste; so bekommen die Ansichten mehr Platz.

## Die Statusleiste und der Ausgabebereich
Die Statusleiste am unteren Rand zeigt die neueste Meldung des Editors. Fehler erscheinen in Rot. Klicken Sie auf die Leiste oder drücken Sie `Ctrl+4`, um den Ausgabebereich mit dem vollständigen Meldungsverlauf zu öffnen. Die meisten Befehle melden dort, was sie getan haben, zum Beispiel wie viele Brushes geschnitten wurden.

## Dialoge und Erscheinungsbild
Dialoge wie Einstellungen, Exportieren und Bild rendern öffnen sich als eigene Fenster. Einstellungen → Ansicht → Farbschema wählt zwischen Der Arbeitsumgebung folgen, Hell und Dunkel. **Deckkraft der Bedienfelder (%)** auf der Seite Übersicht macht die Bedienfelder über den Ansichten durchscheinend. Das Fenster zeichnet seine Schaltflächen im Stil Ihres Systems: nativ unter macOS, im Stil von Windows 11 unter Windows und im GNOME-Stil unter Linux.

# Navigation und Tastatur
## Bewegen in der 3D-Ansicht
Halten Sie die rechte Maustaste gedrückt und ziehen Sie, um sich umzusehen. Ziehen mit der mittleren Taste verschiebt die Ansicht. Das Mausrad bewegt vor und zurück. `W`, `A`, `S` und `D` fliegen nach vorn, links, hinten und rechts, `Q` steigt und `X` sinkt; halten Sie `Shift` für größere Schritte. Ist **Alt + Ziehen mit der mittleren Taste bewegt die Kamera** unter Einstellungen → Maus eingeschaltet, bewegt Alt mit der mittleren Taste entlang der Blickrichtung. Geschwindigkeiten, Invertierungen und das Sichtfeld finden Sie unter Einstellungen → Maus.

## Bewegen in 2D-Ansichten
Ziehen Sie mit der mittleren Taste, um die Ansicht zu verschieben, und zoomen Sie mit dem Mausrad um den Mauszeiger herum. Verschieben und Zoomen einer flachen Ansicht hält die anderen ausgerichtet; das steuert **Die 2D-Ansichten koppeln** unter Einstellungen → Maus. `Ctrl+Alt+Z` setzt den Zoom zurück und `Ctrl+Shift+U` setzt die flachen Ansichten auf den Ursprung zurück.

`Ctrl+U` rahmt die Auswahl in allen Ansichten ein. Ansicht → Gehe zu (`Ctrl+Alt+M`) bewegt die Kamera zu eingegebenen Koordinaten, wählt aus, was in einer bestimmten Zeile der Map-Datei steht, oder verschiebt die Auswahl um einen eingegebenen Betrag.

## Raster und Einrasten
`+` und `-` ändern die Rastergröße, und die Zifferntasten `1` bis `9` wählen direkt eine Größe von 1 bis 256 Einheiten. `0` oder `#` blendet das Raster ein oder aus, und `Alt+0` schaltet das Einrasten ein oder aus. `S` rastet die Eckpunkte der ausgewählten Brushes am aktuellen Raster ein, `Ctrl+Shift+V` rastet sie auf ganze Einheiten und `Ctrl+Alt+Shift+V` am Raster ein.

## Häufige Tastenkürzel
- `F3` oder `Ctrl+Shift+P`: jeden Menübefehl nach Namen suchen und mit `Enter` ausführen. `F10` öffnet die Menüs, die Pfeiltasten bewegen sich darin.
- `Ctrl+S`: Map speichern. `Ctrl+Shift+S`: speichern unter.
- `Ctrl+Z`: rückgängig. `Ctrl+Shift+Z` oder `Ctrl+Y`: wiederholen.
- `Ctrl+C`, `Ctrl+X`, `Ctrl+V`: kopieren, ausschneiden und einfügen. Einfügen setzt die Objekte am Mauszeiger ab; `Ctrl+Alt+V` behält ihre ursprünglichen Koordinaten. Sind Flächen ausgewählt, kopiert Kopieren ihr Material und ihre UV-Einstellungen.
- `Ctrl+D`: duplizieren. `Ctrl+Shift+D`: verknüpftes Duplikat einer Gruppe.
- `Delete` oder `Backspace`: Auswahl löschen.
- `Esc`: bricht das aktuelle Ziehen ab, geht im aktiven Werkzeug einen Schritt zurück, kehrt zu Auswählen zurück und hebt schließlich die Auswahl auf. `Shift+Esc` kehrt direkt zu Auswählen zurück.
- `Ctrl+R`: die letzten wiederholbaren Befehle wiederholen. `Ctrl+Shift+R`: sie vergessen.
- `Ctrl+Alt+0` oder `Ctrl+Alt+Num0`: die Renderkamera an der 3D-Ansicht platzieren.
- `F5`: Materialien neu laden. `F6`: Entity-Definitionen neu laden.

## Tastenkürzel für die Auswahl
- `Ctrl+A`: alles auswählen bzw. alle Anfasser, solange ein Eckpunkt-, Kanten- oder Flächenwerkzeug aktiv ist. `Ctrl+Shift+A`: nichts auswählen. `Ctrl+Alt+A`: Auswahl umkehren.
- `Ctrl+B`: die anderen Objekte in derselben Entität oder Gruppe auswählen.
- `Ctrl+T`: alles auswählen, was die ausgewählten Brushes berühren. `Ctrl+E`: alles auswählen, was in ihnen liegt. `Ctrl+Shift+E`: alles in ihrer Säule entlang der Blickachse auswählen. Die auswählenden Brushes werden dabei verbraucht.
- `Ctrl+I`: Auswahl isolieren. `Ctrl+Alt+I`: Auswahl ausblenden. `Ctrl+Shift+I`: wieder alles anzeigen.
- `Ctrl+G`: gruppieren. `Ctrl+Shift+G`: Gruppierung aufheben.

## Tastenkürzel für Transformationen
- Die Pfeiltasten und `Page Up` / `Page Down` verschieben die Auswahl um einen Rasterschritt in die Richtung, in die sie in der aktiven Ansicht zeigen. Mit `Ctrl` duplizieren sie beim Verschieben, mit `Alt` drehen sie.
- `Alt+X`, `Alt+Y`, `Alt+Z`: eine Vierteldrehung um diese Achse; mit `Shift` in die andere Richtung.
- `Ctrl+F`: quer zur Ansicht spiegeln; `Ctrl+Alt+F`: oben und unten vertauschen. `Ctrl+Shift+X` und `Ctrl+Shift+Y` spiegeln entlang X bzw. Y.
- `Insert`: eine Punkt-Entität im Mittelpunkt der Auswahl platzieren.
- `Tab`: bei ausgewähltem Brush reihum Auswählen → Eckpunkt → Kante → Fläche → Auswählen.

## Die Blender-Tastenbelegung
Einstellungen → Tastatur → Tastenbelegung bietet Heron Brush (Standard, in diesem ganzen Handbuch verwendet) und Blender. Mit Blender gilt: `G` wählt aus und verschiebt, `R` dreht, `S` skaliert, `E` zieht (Ziehen), `1`, `2` und `3` wählen die Werkzeuge Eckpunkt, Kante und Fläche, `K` schneidet, `B` wählt aus, `A` wählt alles aus und `Alt+A` hebt die Auswahl auf, `X` löscht, `Shift+D` dupliziert, `H` blendet aus und `Alt+H` zeigt alles wieder an, und `Ctrl+Shift+Z` wiederholt. Alles andere ist in beiden gleich.

# Brushes und Modellierwerkzeuge
Brushes sind konvexe Körper, die durch Ebenen definiert sind. Bauen Sie komplexe Objekte aus mehreren einfachen Brushes, statt zu versuchen, einen einzelnen Brush konkav zu machen. Halten Sie die Zahl der Brushes niedrig, wo eine einfache Form genügt.

## Brushes zeichnen
Wählen Sie das Brush-Werkzeug (`B`) und ziehen Sie in einer Ansicht. Die Werkzeugleiste zeigt **Zu zeichnende Form**: Quader, Treppe, Torbogen, Zylinder, Kegel, Sphäroid (UV), Sphäroid (Ikosaeder) und 3D-Text einfügen. `Alt+1` bis `Alt+7` wählen eine Form direkt. Je nach Form zeigt die Leiste **Aushöhlen** und **Dicke:**, für Treppen **Stufenhöhe:** und **Richtung:**, für runde Formen **Achse:**, **Seiten:** und den Kreismodus, für Kugeln **Ringe:** und für das Ikosaeder **Genauigkeit:**. Stellen Sie die Optionen vor dem Ziehen ein. Prüfen Sie das Ergebnis in mehr als einer Ansicht.

## Auswählen
Klicken Sie auf einen Brush, um ihn auszuwählen, und klicken Sie mit `Ctrl`, um ihn zur Auswahl hinzuzufügen. Ziehen von einer leeren Stelle aus zieht einen Auswahlrahmen auf. In der 3D-Ansicht wählt `Shift`-Klick eine einzelne Fläche aus, und `Ctrl+Shift`-Klick fügt weitere Flächen hinzu. Ziehen an einem ausgewählten Brush verschiebt ihn; Ziehen an einem nicht ausgewählten wählt nur diesen aus und verschiebt ihn.

In der 3D-Ansicht extrudiert `Shift`-Ziehen an einer Fläche eines ausgewählten Brushes diese Fläche. `Alt`-Klick auf eine andere Fläche überträgt Material und Ausrichtung der ausgewählten Fläche darauf; `Alt+Ctrl` überträgt nur das Material und `Alt+Shift` die Drehung.

## Formwerkzeuge
- **Auswählen** (`Esc`): auswählen und verschieben.
- **Eckpunkt** (`V`), **Kante** (`E`), **Fläche** (`F`): Ecken, Kanten oder Flächen ziehen. `Shift`-Ziehen von der Mitte einer Kante oder Fläche aus fügt dort eine Ecke hinzu. `Delete` entfernt die gewählten Ecken.
- **Drehen** (`R`), **Skalierung** (`T`), **Scheren** (`G`): die Auswahl mit Anfassern transformieren.
- **Schneiden** (`C`): klicken Sie zwei oder drei Punkte, um eine Schnittebene festzulegen. `Enter` schneidet, `Ctrl+Enter` wählt, welche Seite erhalten bleibt, und `Backspace` oder `Esc` entfernt den letzten Punkt.
- **Zusammenbauen** (`N`): setzen Sie Punkte rund um ein Volumen und drücken Sie `Enter`, um daraus einen konvexen Brush zu erzeugen. `Backspace` entfernt den letzten Punkt.
- **Ziehen** (`Y`): siehe den Abschnitt zu Ziehen weiter unten.
- **Kontrollpunkt** (`P`): das Kontrollgitter eines Patches umformen.
- **Verrundung** (`O`): klicken Sie auf eine Kante, um sie abzurunden, oder wählen Sie Kanten oder Eckpunkte mit deren Werkzeugen und drücken Sie `Enter`. Mehr Flächen ergeben ein glatteres Ergebnis und mehr Geometrie.
- **Streichen** (`K`): siehe Materialien und UVs.

## Brushes kombinieren
- `Ctrl+J`: die ausgewählten Brushes zu ihrer konvexen Hülle verschmelzen.
- `Ctrl+K`: die ausgewählten Brushes von den Brushes abziehen, die sie berühren.
- `Ctrl+Shift+K`: die Auswahl aushöhlen, mit Wänden von einem Rasterschritt Dicke.
- `Ctrl+L`: die Schnittmenge der Auswahl bilden.

## Ziehen, Verrundung, Patch und Abschrägung
**Ziehen** erzeugt Geometrie aus ausgewählten Flächen entlang eines Pfads. Wählen Sie die Flächen aus (im Werkzeug Ziehen per `Alt`-Klick), stellen Sie dann im Abschnitt Ziehen des Tabs Fläche **Weg**, **Drehung** (Achse und Winkel), **Skalierung**, **Schritte** und **Pfad** (Ausrichtung Ganzzahlig oder Frei; Bogen, Gerade oder S-Kurve) ein und drücken Sie **Ziehen**. Die Zeile unter den Bedienelementen sagt, wie viele Brushes entstehen.

Der Abschnitt **Verrundung** rundet die gewählten Kanten mit **Radius**, **Flächen** und **Ecken** (Auf Gehrung oder Gerundet) ab; eine einzelne Fläche ergibt eine Fase.

**Patch** zeigt **Spalten** und **Zeilen** des ausgewählten Patches mit Schaltflächen, um ein Paar hinzuzufügen oder zu entfernen. Geometrie → Flächen in Patches umwandeln (`Ctrl+P`) erzeugt Patches aus ausgewählten Flächen.

**Einzug & Abschrägung** arbeitet mit ausgewählten Flächen: **Einzug** zieht die Fläche von ihren Kanten nach innen und **Tiefe** schiebt sie nach außen (oder nach innen, wenn negativ). **Flächen abschrägen** wendet beides an; die dritte Schaltfläche fügt je nach Vorzeichen von Tiefe eine Platte hinzu oder schneidet eine Vertiefung aus. **Ecke verbinden** öffnet einen Dialog, der zwei Brushes mit Vorschau an einer Gehrungs- oder abgerundeten Ecke verbindet.

## 3D-Text
Geometrie → 3D-Text einfügen (auch der letzte Eintrag in der Formenliste des Brush-Werkzeugs) erzeugt Brushes aus Text. Es verwendet Heron Block, eine eingebaute Polygonschrift mit abgeschnittenen Ecken, proportionalen Breiten und tschechischen Akzenten; Kleinbuchstaben verwenden die Formen der Großbuchstaben. Geben Sie den **Text** ein, stellen Sie **Höhe** (die Versalhöhe; Akzente ragen darüber hinaus) und **Tiefe** ein, beobachten Sie die Live-Vorschau und drücken Sie **Einfügen**.

Wenn eine Operation keine gültige Geometrie erzeugen kann, passen Sie ihre Abmessungen an oder machen Sie den Schritt rückgängig und vereinfachen Sie die Form.

## Arrays
Geometrie → Array kopiert die Auswahl und zeigt die Kopien in der Karte, während du die Werte änderst. Die Anzahlen schließen das Original ein, und ein Array erzeugt höchstens 1024 Kopien.

- **Reihe** verschiebt jede Kopie gegenüber der vorigen um **Schritt X**, **Schritt Y** und **Schritt Z** und dreht sie um **Drehung pro Kopie (Grad)** um die gewählte Achse.
- **Raster** füllt **Anzahl X**, **Anzahl Y** und **Anzahl Z** Elemente im Abstand **Schritt X**, **Schritt Y** und **Schritt Z**.
- **Ring** verteilt die Elemente über **Winkel (Grad)** um einen Mittelpunkt im Abstand **Radius (Einheiten)** von der Auswahl. Mit **Zur Mitte ausrichten** dreht sich jede Kopie mit dem Ring.
- **Streuung** setzt die Kopien an zufällige Stellen innerhalb des Radius, quer zur gewählten Achse. Derselbe **Seed** ergibt dieselbe Anordnung, und **Zufällige Drehung** dreht jede Kopie.

Ist eine ganze Gruppe ausgewählt, macht **Verknüpfte Kopien** jede Kopie zu einem verknüpften Duplikat, sodass eine Änderung an einer alle erreicht. **Anwenden** behält die Kopien und wählt sie mit dem Original aus; **Abbrechen** entfernt sie. Das ganze Array wird in einem Schritt rückgängig gemacht.

## Gelände modellieren
Gelände besteht aus vielen kleinen Brushes, deren Oberfläche der Modellierpinsel hinausschiebt und hineindrückt. Wähle einen oder mehrere Brushes aus und dann Geometrie → In Säulen teilen und danach die Form der Säulen: Dreieck, Quadrat oder Sechseck. Das zerschneidet sie entlang der Rasterlinien in Säulen von einem Rasterschritt Breite und legt die Säulen in eine Gruppe namens Terrain. Die Schnitte folgen dem Raster der Welt, daher werden nebeneinanderliegende Brushes an denselben Stellen geschnitten und teilen sich ihre Ecken. Jede Säule steht entlang der dünnen Seite des Brushes: senkrecht in einem Boden oder einer Decke, waagerecht in einer Wand. Eine Höhle funktioniert genauso: Höhle einen Quader aus (Hollow selection) mit einer Wandstärke von einem Rasterschritt, wähle die Wände aus und teile sie.

Das Fenster **Modellieren**, das die Schaltfläche am Ende der Modellierleiste öffnet, kann die Auswahl auch teilen, und zwar in die Form aus seiner Zeile Säulenform. Beide Stellen merken sich die zuletzt verwendete Form. **Dreieck** erzeugt zwei pro Rasterzelle, und die Oberseite eines Dreiecks bleibt eben, wie auch immer du sie modellierst. **Quadrat** erzeugt eine pro Zelle, also halb so viele Brushes; ihre Oberseite knickt entlang einer Diagonale. **Sechseck** belegt den Brush mit Sechsecken, zwei Rasterschritte breit, jedes in sechs Dreiecke um seine Mitte geteilt, sodass jede Oberseite eben bleibt.

Wähle das Werkzeug **Modellieren** (`J`) und ziehe in der 3D-Ansicht über die Oberfläche. Hältst du die Maustaste gedrückt, ohne die Maus zu bewegen, arbeitet der Pinsel an derselben Stelle weiter, zehnmal pro Sekunde, sodass ein Hügel wächst, solange du sie hältst. Der orange Ring zeigt den Pinsel. Ohne Auswahl ändert der Pinsel jeden Brush darunter, mit Auswahl nur die ausgewählten. Die Leiste oben im Fenster legt fest, was ein Strich tut:

- **Anheben** und **Senken** bewegen die Oberfläche nach außen oder innen, am stärksten in der Mitte des Rings und am Rand gar nicht. Halte `Shift`, um sie zu tauschen.
- **Glätten** gleicht Unebenheiten aus.
- **Ebnen** ebnet die Oberfläche auf den Punkt ein, an dem der Strich begann.

Im selben Fenster legt **Richtung** fest, wohin:

- **Nach oben** bewegt die Oberfläche senkrecht nach oben und unten, innerhalb eines Kreises auf dem Boden. Das ist für Gelände gedacht.
- **Kamera** bewegt sie auf die Ansicht zu und von ihr weg, innerhalb einer Kugel um den Mauszeiger. Schau auf einen Boden, eine Wand oder eine Decke, und der Pinsel formt genau das.
- **Normale** bewegt sie aus der Fläche heraus, die beim Beginn des Strichs unter dem Mauszeiger liegt.

Die zwei Zahlen sind **Radius (Einheiten)** und **Stärke (%)**. Nur die Ecken auf der Seite, die der Strich bewegt, bewegen sich; die Rückseite bleibt, wo sie ist, und eine Säule bleibt mindestens eine Einheit dick. Wo sich Brushes eine Ecke teilen, bewegen sie sie gemeinsam, sodass die Oberfläche geschlossen bleibt. Ein Strich wird in einem Schritt rückgängig gemacht. In den 2D-Ansichten wählt das Werkzeug aus wie das Auswahlwerkzeug.

Wenn das Modellieren fertig ist, wähle das Terrain aus und dann Geometrie → **Säulen zusammenfügen** oder die Schaltfläche im Fenster **Modellieren**. Benachbarte Säulen, die zusammen einen konvexen Brush ergeben und gleich aussehen, werden zu einem, sodass flache Stellen zu wenigen großen Brushes werden. Die Oberfläche bewegt sich nicht, und das Zusammenfügen lässt sich in einem Schritt rückgängig machen.

**Brushes verbinden**, gleich daneben im Menü und im Fenster, fügt genauso zusammen, achtet aber nicht auf das Aussehen der Flächen: Wo zwei Flächen zu einer werden, bekommt sie das Material einer der beiden. Es funktioniert mit beliebigen ausgewählten Brushes, nicht nur mit Terrain. Vier Quader in L-Form werden zu zwei, nie zu einem, der das L ausfüllt, und ein ganzes modelliertes Terrain schrumpft auf so wenige Brushes, wie das Zusammenfügen von Nachbarn finden kann. Zusammengeführt werden nur Brushes in derselben Gruppe oder Entität, sodass eine Tür nie die Wand neben sich schluckt.

Für Terrain wie in Voxel-Spielen teile in **Quadrat**-Säulen und wähle im Fenster **Modellieren** unter **Strich** die Option **Blöcke**. Ein Strich bewegt dann die ganze Oberseite jeder Säule in ganzen Rasterschritten und lässt die Seiten senkrecht, sodass das Gelände in Stufen ansteigt. Alle Modi wirken wie zuvor; kleine Tupfer summieren sich, bis eine Säule einen ganzen Schritt gestiegen ist. Säulen anderer Form bleiben, wie sie sind.

**Frei** braucht keine Säulen und wirkt auf beliebige Brushes. Anheben fügt in einer Kugel um den Zeiger Gelände hinzu, Senken nimmt es weg, sodass Überhänge und Höhlen entstehen können. Glätten ebnet es, Ebnen bringt es auf die Höhe, wo der Strich begann. Jeder Tupfer baut die erreichten Brushes auf dem Raster gerundet neu auf und lässt alles außerhalb seiner Kugel, wie es war, scharfe Kanten eingeschlossen. Das Ergebnis sind gewöhnliche Brushes, auf die jedes andere Werkzeug wirkt, und **Säulen zusammenfügen** macht sie danach weniger. Ein feineres Raster gibt eine glattere Oberfläche und mehr Brushes.

# Materialien und UVs
## Materialien laden und zuweisen
Öffnen Sie im Tab Fläche den Abschnitt **Materialien**. **Einen Ordner laden** fügt einen Ordner mit Bildern hinzu und **Eine .wad laden** ein Texturpaket; beide erscheinen unter **Sammlungen**, wo das Kreuz einen Eintrag entfernt. Tippen Sie in **Materialien filtern**, um die Liste einzugrenzen. Klicken Sie auf ein Material, um es zum aktuellen zu machen; **Auf die Auswahl anwenden** legt es dann auf jede ausgewählte Fläche. **Auswählen** bietet Brushes damit auswählen und Flächen damit auswählen. **Ersetzen** tauscht ein Material gegen ein anderes aus, in der ganzen Map oder nur in der Auswahl.

Das Werkzeug **Streichen** (`K`) legt das aktuelle Material auf die Fläche unter dem Mauszeiger: Ein Klick bemalt eine Fläche, ein Doppelklick den ganzen Brush, und `Shift`-Klick übernimmt das Material der Fläche als aktuelles, wie eine Pipette.

## Der UV-Editor
Der Abschnitt **UV** im Tab Fläche zeigt Material und Umriss der ausgewählten Fläche. Maussteuerung auf der Arbeitsfläche:
- Das Mausrad zoomt um den Mauszeiger herum. Ziehen mit der rechten oder mittleren Taste verschiebt die Ansicht. Ein Doppelklick mit der rechten Taste rahmt die Fläche wieder ein; wählen Sie eine andere Fläche aus, wird sie automatisch eingerahmt.
- Ziehen mit der linken Taste verschiebt das Material, während der Umriss der Fläche an seinem Platz bleibt. Der Ring dreht die Zuordnung, die Eckanfasser skalieren sie und der mittlere Anfasser legt den Ursprung dieser Änderungen fest.
- `Alt`-Ziehen mit der linken Taste an einer Kachelkante schert, ebenso `Shift`-Ziehen mit der mittleren Taste.
- Jedes Ziehen ist ein Rückgängig-Schritt, und `Esc` bricht ein laufendes Ziehen ab.

Unter der Arbeitsfläche liegen **Versatz**, **Skalierung**, **Winkel** und, wenn das Spiel sie verwendet, **Inhalts-Flags**, **Oberflächen-Flags** und **Oberflächenwert**. Die Schaltflächen sind **Zurücksetzen**, **An Weltachsen ausrichten**, **Einpassen**, **H spiegeln**, **V spiegeln** und die beiden 90°-Drehungen. **Ausrichten** dreht das Material entlang einer der Kanten der Fläche (erneut drücken für die nächste Kante, mit `Shift` für die vorherige). Die vier Bündigkeitsschaltflächen schieben das Material an eine Kante. **U einpassen** und **V einpassen** skalieren so, dass eine ganze Zahl von Wiederholungen die Fläche bedeckt; erneut drücken für die nächste Anzahl, und halten Sie `Ctrl`, um die Unterteilung eines Trim Sheets beizubehalten.

## UV-Tasten
Bei ausgewählten Flächen verschieben die Pfeiltasten das Material, `[` und `]` drehen es um 15°, `,` und `.` halbieren oder verdoppeln seine Skalierung, `Ctrl+F` und `Ctrl+Alt+F` spiegeln es, `Shift+R` setzt es zurück, `Alt+Shift+R` richtet es an den Weltachsen aus und `Ctrl+Shift+F` passt es in jede Fläche ein.

**Sperre** in der Werkzeugleiste behält die Zuordnung bei, während Brushes verschoben werden, und **UV** (`U`) behält sie bei, während Brushes gedreht werden.

## Werkzeugmaterialien
- skip: entfernt die markierte Fläche aus Engine-Exporten.
- clip: erzeugt Kollision ohne sichtbare Geometrie.
- origin: markiert den Drehpunkt einer Brush-Entität.

Die Namen lassen sich im Exportfenster auf der Karte **Werkzeugmaterialien** ändern. Verwenden Sie skip für Flächen, von denen Sie wissen, dass sie verdeckt sind.

## Flächen vom Export auslassen
Wählen Sie eine Fläche aus und schalten Sie im Abschnitt UV **Fläche beim Export auslassen** ein. Eine orangefarbene Schraffur kennzeichnet sie in der UV-Vorschau. Ihr Material und ihre Bemalung bleiben gespeichert, und der Brush bleibt geschlossen und bearbeitbar. Exportierte Render-Meshes lassen diese Fläche weg, die Kollision bleibt erhalten. Lässt man ein Viereck eines Würfels weg, entstehen zwei Render-Dreiecke. Dadurch werden Flächen weder doppelseitig noch Meshes zusammengeführt.

**Verdeckte Flächen auslassen** markiert Flächen, die vollständig von statischen Brushes bedeckt sind, einschließlich identischer, sich berührender Flächen und eingeschlossener Brushes. Es wirkt auf die ausgewählten Brushes oder, wenn nichts ausgewählt ist, auf die gesamte sichtbare, nicht gesperrte Weltgeometrie. Brush-Entitäten, transparente Materialien und Brushes, die bereits ausgelassene Flächen haben, werden übersprungen. Teilweise verdeckte Flächen werden nicht geteilt. Ein einziges Rückgängig stellt alles wieder her. Ansicht → Ausgelassene Flächen anzeigen blendet solche Flächen in den Editor-Ansichten aus oder ein; die Auswahl per Klick verwendet weiterhin den ganzen Brush.

# Farbe und Verläufe
Der Abschnitt **Farbe** im Tab Fläche bemalt Flächen mit Volltonfarben oder Verläufen über ihren Materialien. Das ist nützlich, um ein Level grob aufzubauen (Blockout), und für stilisierte Grafik. Bemalte Flächen behalten in den Ansichten ihre echten Farben; die Auswahl wird durch ihren Umriss angezeigt.

## Palette und Mischfarbe
Klicken Sie auf eines der zwölf Farbfelder der Palette, um die ausgewählten Flächen oder ganze ausgewählte Brushes damit zu bemalen. **Deckkraft** legt in Prozent fest, wie deckend die nächste Bemalung ist. **Mischen** hat eine Farbschaltfläche, die den Farbwähler öffnet; ziehen Sie im Farbwähler, und die Auswahl folgt live, oder drücken Sie **Streichen**, um die Mischfarbe in einem Schritt anzuwenden. **Zurück zum Material** entfernt die Bemalung, sodass die Fläche wieder ihr Material zeigt. Die Zeile oben im Abschnitt sagt, was ausgewählt ist und was es trägt.

## Der Farbwähler
Jede Farbschaltfläche im Editor öffnet denselben Farbwähler. Ziehen Sie im Quadrat, um Sättigung und Helligkeit zu wählen, und im Streifen daneben, um den Farbton zu wählen. Darunter liegen die Werte **R**, **G** und **B** und, wo die Farbe eine hat, ein Schieberegler **Deckkraft**. Das Hex-Feld akzeptiert `#RRGGBB` oder `#RRGGBBAA`; es wird rot, solange der Text keine gültige Farbe ist. **Kopieren** legt den Hex-Code in die Zwischenablage. Klicken Sie außerhalb des Farbwählers, um ihn zu schließen.

## Verläufe
**Verlauf** wählt Linearer Verlauf oder Radialer Verlauf, und die Schaltfläche daneben kehrt die Farben um. Jeder Farbpunkt hat eine Farbschaltfläche und **Position (%)**, und Punkte zwischen den Enden haben eine Schaltfläche zum Entfernen. **Farbpunkt hinzufügen** fügt einen Punkt in der größten Lücke ein; ein Verlauf kann bis zu acht Punkte haben. **Verlaufswinkel** legt die Richtung eines linearen Verlaufs fest, mit den Schaltflächen **+45°** und **−45°**.

Unter der Vorschau liegt eine Leiste mit einer Markierung für jeden Farbpunkt. Klicken Sie auf die Leiste, um einen Punkt hinzuzufügen, klicken Sie auf eine Markierung, um ihren Farbwähler zu öffnen, ziehen Sie eine Markierung, um sie zu verschieben, und klicken Sie mit der rechten Maustaste darauf, um sie zu entfernen. Die drei Farbfelder unter der Leiste sind Vorlagen: Warmer Verlauf, Kühler Verlauf und Graustufenverlauf. Änderungen an einem Verlauf wirken live auf die Auswahl.

Die Richtung eines Verlaufs ist wichtig: Richten Sie ihn entlang einer Dachschräge oder eines gebogenen Objekts aus, nicht quer über Flächen, die nichts miteinander zu tun haben. Prüfen Sie das Modell nach dem Bemalen von beiden Seiten.

## Oberflächenmaterial
**Oberflächenmaterial** legt **Rauheit** (0 ist glänzend, 100 ist matt), **Metallisch** (0 für Holz, Stein oder Kunststoff, 100 für Metall) und **Emission** (0 bis 8; die Oberfläche leuchtet in ihrer eigenen Farbe, auch bei Verläufen, beleuchtet aber keine Objekte in der Nähe) fest. Änderungen wirken live auf die ausgewählten Flächen oder Brushes. **Standardoberfläche verwenden** entfernt die Überschreibung. Alle Änderungen lassen sich rückgängig machen.

Oberflächeneffekte sind in der beleuchteten 3D-Ansicht und in Kamera-Renderings zu sehen. Die flache Darstellung und das Drahtgitter lassen die Glanzlichter bewusst weg. Bloom kann ein Leuchten um emittierende Oberflächen hinzufügen. Godot und glTF/GLB erhalten die Materialwerte, und der Unity-Importer wandelt die Rauheit in Glätte (Smoothness) um. Die Grenzen der einzelnen Ziele beschreibt `docs/surface-materials.md`.

# Assets, Ebenen und Gruppen
## Die Asset-Bibliothek
Öffnen Sie den Tab **Assets**. Fügen Sie mit **Ordner hinzufügen** einen Ordner mit `.map`-Modellen hinzu; die eingebaute Bibliothek ist immer vorhanden. Tippen Sie in **Assets filtern**, um zu suchen. Klicken Sie auf ein Modell, um es auszuwählen, und doppelklicken Sie dann darauf, drücken Sie `Enter` oder klicken Sie auf **Einfügen**, um es dort einzufügen, wo der Mauszeiger zuletzt war. Sie können ein Modell auch in eine Ansicht ziehen. Asset-Brushes werden als eine Gruppe eingefügt, damit sich das Modell als Ganzes bewegt.

**Auswahl als Asset speichern** schreibt die ausgewählten Brushes als `.map` in die Bibliothek. Bibliotheks-Assets sind gewöhnliche, bearbeitbare Brushes, keine importierten Meshes.

## Ebenen
Die Liste **Ebenen** im Tab Map gliedert große Bereiche. Geben Sie einen Namen ein und klicken Sie auf **Hinzufügen**, um eine Ebene anzulegen, oder drücken Sie `Enter`, um die aktuelle umzubenennen. Jede Ebene lässt sich ausblenden, sperren oder von Exporten ausnehmen. **Hierher verschieben** verschiebt die Auswahl in die aktuelle Ebene. Das Kontextmenü in den Ansichten bietet außerdem In Ebene verschieben, Als aktive Ebene setzen, Ebenen ausblenden und Ebenen isolieren.

## Gruppen und verknüpfte Gruppen
`Ctrl+G` gruppiert die Auswahl und `Ctrl+Shift+G` hebt die Gruppierung auf. Der Abschnitt **Gruppen** im Tab Map listet die Gruppen auf und bietet **Öffnen**, **Umbenennen**, **Zusammenführen**, **Gruppierung aufheben**, **Herausnehmen** und **Herauslösen**. Eine verknüpfte Gruppe ist ein Satz von Kopien, die Änderungen teilen: `Ctrl+Shift+D` erstellt ein verknüpftes Duplikat, Geometrie → Verknüpfte Gruppen abgleichen aktualisiert die Kopien, und **Verknüpfung lösen** macht eine Kopie unabhängig. Im Tab Entity kann eine Eigenschaft als eigene Eigenschaft einer Kopie erhalten bleiben, wenn verknüpfte Gruppen abgeglichen werden.

## Ein gemeinsamer Maßstab
Der Standard-Exportmaßstab beträgt 32 Map-Einheiten pro Meter. Die eingebaute Markierung für den Spielerstart ist 64 Einheiten hoch, eine Referenz von 2 Metern. Vergleichen Sie Türöffnungen, Fahrzeuge, Möbel und Pflanzen damit. Ändern Sie den Exportmaßstab nur bewusst und halten Sie zusammengehörige Assets einheitlich.

# Entitäten und Gameplay
Gängige Entity-Definitionen sind eingebaut und immer verfügbar: Lichter, Spielerstarts, Türen, Schalter, Plattformen, Trigger, statische Requisiten, Sounds und Wegpunkte. Wenn Sie über **Entity-Definitionen** im Tab Entity eine Projekt-FGD laden, ersetzt sie Klassen gleichen Namens und behält die übrigen eingebauten.

## Entitäten platzieren
Tippen Sie im Browser **Entities** des Tabs Entity in **Klassen filtern** und wählen Sie eine Klasse. Bei einer Punktklasse wie einem Licht klicken Sie auf **Setzen** oder doppelklicken auf die Klasse; `Insert` setzt eine im Mittelpunkt der Auswahl. Bei einer Brush-Klasse wie einer Tür oder einem Trigger wählen Sie die Brushes aus und klicken auf **Erzeugen**. Klassen lassen sich auch in eine Ansicht ziehen.

## Eigenschaften
Die Eigenschaftsliste bearbeitet die ausgewählte Entität. Entitäten derselben Klasse werden gemeinsam bearbeitet. **Eigenschaft hinzufügen** fügt einen Schlüssel hinzu, den die Definition möglicherweise nicht kennt; **Standardwerte** kann vorhandene Eigenschaften auf ihre Standardwerte zurücksetzen oder fehlende hinzufügen. Eigenschaften, die die Klasse definiert, die Entität aber nicht hat, werden abgeblendet angezeigt; wenn Sie einen Wert eingeben, werden sie hinzugefügt. Verknüpfen Sie Entitäten mit targetname und target.

## Was ein Export leisten kann
Die Direktiven von Godot erzeugen native Knoten, Kollisionskörper und Trigger-Bereiche. Unity und Unreal unterstützen native Standardlichter, und Unreal erzeugt außerdem PlayerStart-Actors. Andere Daten bleiben erhalten, soweit der Importer des Ziels es zulässt.

Eine Tür-Entität allein fügt kein Spielverhalten hinzu. Bewegung, Reaktionen auf Trigger, Spielersteuerungen und Projektskripte müssen weiterhin in der Engine umgesetzt werden. Defold exportiert Entitätseigenschaften und Kollisionsobjekte; Flax platziert Modelle und generische Entitäten. Prüfen Sie das Ergebnis in Ihrer Engine, bevor Sie Gameplay darauf aufbauen.

# Animation
Heron Brush kann einfache Bewegungen als Vorschau abspielen und Schlüsselbildanimationen von Brush-Entitäten bearbeiten und sie dann in mehrere Engines exportieren.

## Animationsvorschau
Schalten Sie **Animationsvorschau** im Menü Ansicht, im Menü Animation, im Menü Rastereinstellungen oder im Abschnitt Ansicht des Tabs Map ein. Die Vorschau spielt die Drehung von func_rotate und die Hin-und-her-Bewegung von func_shuttle ab, wenn start_on aktiviert ist, und verwendet dabei die Eigenschaften axis, speed, direction, travel und wait der Entität; außerdem spielt sie Schlüsselbild-Clips ab, für die Beim Start abspielen gesetzt ist. Die Assets Ventilator und Windmühle enthalten bereits rotierende Teile.

Schalten Sie die Vorschau aus, wird die ursprüngliche Pose wiederhergestellt. Ein Klick in eine Ansicht stoppt die Wiedergabe, bevor ausgewählt oder bearbeitet wird; die Kameranavigation funktioniert weiter. Die Vorschau verändert weder die Map noch den Rückgängig-Verlauf oder Exporte, und sie führt weder Physik noch Spiellogik aus.

## Der Schlüsselbild-Editor
Öffnen Sie ihn über Animation → Schlüsselbildanimation oder über das Menü Rastereinstellungen. Er wird unter den Ansichten angedockt oder im 3D-Fenster, solange dieses separat ist. Das Fenstersymbol in seiner Kopfzeile verschiebt ihn in ein eigenes Fenster; dabei bleiben das Bild und der Rückgängig-Verlauf erhalten. **Animation bearbeiten** schaltet die Ansichten auf die animierte Pose um, **Karte bearbeiten** kehrt zur Ruhegeometrie zurück. Im Animationsmodus zeigt jede Ansicht unten **Animation bearbeiten** und die Bildnummer.

Wählen Sie eine Brush-Entität oder einen Brush bzw. eine Brush-Gruppe aus, um sie zu animieren. Eine flache Brush-Gruppe teilt sich eine Animation und einen Drehpunkt; Gruppen mit mehreren Entitäten brauchen getrennte Animationen.

## Clips
Die Auswahl **Animation** wechselt zwischen benannten Clips. Geben Sie in **Animationsname** einen Namen ein und klicken Sie auf **Speichern als**, um den aktuellen Clip unter diesem Namen zu kopieren; **Löschen** entfernt einen Clip. **Verschieben** erzeugt einen Clip `translate` ohne Drehung. **Türanimationen** erzeugt weiche Clips `open` und `close` mit einer 90-Grad-Drehung um den aktuellen Drehpunkt; verschieben Sie den Drehpunkt vorher an das Scharnier. **Beim Start abspielen** lässt den Clip beim Start des Levels abspielen; die Türvorlage schaltet das aus.

## Transport und Wiedergabe
Die Transportschaltflächen sind **Anfang**, **Vorheriger Key**, **Wiedergabe**, **Nächster Key** und **Ende**. **Bild** zeigt und setzt das aktuelle Bild, und **Dauer (s)** legt die Länge des Clips fest. Der Modus ist **Einmal**, **Schleife** oder **Hin und zurück**.

## Die Zeitleiste
Die Zeitleiste hat ein Lineal mit dem Abspielkopf und eine Spur für **Position** (X, Y und Z werden gemeinsam gekeyt) sowie eine für jeden Drehwinkel, RX, RY und RZ. Klicken oder ziehen Sie im Lineal, um den Abspielkopf zu bewegen und diese Pose in allen Ansichten anzuzeigen. Ziehen mit der mittleren Taste verschiebt die Zeitleiste. Ziehen Sie Keys, um sie zeitlich zu verschieben; `Shift` erweitert die Auswahl; ein Doppelklick auf eine Spur fügt einen Key ein; `Ctrl`+Mausrad zoomt. **Kurven** zeigt statt der Spuren die Wertkurven.

Die Schaltflächen darüber sind **Key hinzufügen**, **Entfernen**, **Kopieren**, **Einfügen** und **Duplizieren**. **An Frames einrasten** hält Keys auf ganzen Bildern, und **Einpassen** zeigt den ganzen Clip. **Schlüssel neu takten** öffnet **Zeitskalierung**, **Versatz (Frames)**, **Ab Abspielkopf skalieren**, **Dauer verlängern** und **Auswahl neu takten**.

Der Key-Inspektor neben der Zeitleiste bearbeitet den ausgewählten Key: **Bild**, **Wert** (bzw. die drei Koordinaten eines Position-Keys), die Interpolation **Linear**, **Konstant** oder **Bézier**, **Eingehende Steigung** und **Ausgehende Steigung** sowie **Tangenten glätten**.

## Tasten im Editor
Solange die Zeitleiste den Tastaturfokus hat: `Space` spielt ab oder pausiert, die Pfeiltasten links und rechts gehen ein Bild weiter, `Ctrl+A` wählt alle Keys aus, `Ctrl+C`, `Ctrl+X` und `Ctrl+V` kopieren, schneiden aus und fügen Keys ein, `Ctrl+D` dupliziert sie, `Delete` entfernt sie, `Ctrl+Z` macht rückgängig und `Esc` bricht ein Ziehen ab. Außerhalb der Zeitleiste verlässt `Esc` den Animationsmodus. Rückgängig lässt Zoom, Auswahl und Abspielkopf, wo sie waren.

## Posieren in den Ansichten
Die Zeile unter der Zeitleiste enthält **Zwiebelschalenansicht** mit einer Anzahl und **Abstand**, die benachbarte Posen als Geister anzeigt; die Koordinaten des **Drehpunkt**; **Automatische Schlüssel**; und die Posenwerkzeuge **Verschieben**, **Drehen** und **Drehpunkt**.

- **Key hinzufügen** schaltet Automatische Schlüssel ein und pausiert die Wiedergabe. Ist Automatische Schlüssel eingeschaltet und **Verschieben** gewählt, ziehen Sie in der 3D-Ansicht an einer farbigen Achse des Anfassers der ausgewählten Entität, um ihre Position im aktuellen Bild zu keyen.
- Ziehen Sie in einer 2D-Ansicht den ausgewählten Brush oder ein Mitglied seiner Gruppe, um die Verschiebung entlang der beiden sichtbaren Achsen zu keyen, mit Einrasten am Raster. Automatische Schlüssel muss dafür nicht eingeschaltet sein.
- Mit **Drehen** ziehen Sie den Brush in einer 2D-Ansicht um seine orangefarbene Drehpunktmarkierung. Oben keyt RZ, Vorne keyt RY und Seite keyt RX, eingerastet auf Rasterwinkel.
- Mit **Drehpunkt** ziehen Sie das gelbe Drehzentrum in einer beliebigen 2D-Ansicht. Es rastet am Raster ein und behält seine Tiefe. Keys und Ruhegeometrie bleiben unverändert.

Der cyanfarbene Pfad in 2D und 3D zeigt den Weg des Drehpunkts. Klicken Sie auf einen Pfadpunkt, um seine Zeit und seine Keys auszuwählen, oder ziehen Sie ihn, um die Position zu ändern; mit eingeschaltetem Automatische Schlüssel können Sie Pfadpunkte auch in der 3D-Ansicht ziehen. Beschriftungen zeigen Bildnummern, gestrichelte Abschnitte markieren Sprünge. `Esc` bricht jedes dieser Ziehen ab, und ein einziges Rückgängig macht ein ganzes Ziehen rückgängig.

Neue Animationen beginnen in Ruhelage mit Keys bei Zeit null. Bewegung entsteht, indem Sie die Pose in einem anderen Bild ändern. Für einen scharfen L-förmigen Pfad keyen Sie den Anfang, die Ecke und das Ende und verwenden Linear mit Hin und zurück.

## Schalter mit Animationen verbinden
Wählen Sie ein func_button, trigger_multiple oder trigger_once aus. Der Tab Entity zeigt **Animationsauslöser** über den Eigenschaften. Wählen Sie ein Ziel aus der Liste, oder klicken Sie auf **Ziel im Ansichtsfenster wählen** und dann in einer beliebigen Ansicht auf ein animiertes Objekt; `Esc` bricht die Auswahl ab. Fehlende Ziele oder Clips werden unter den Bedienelementen gemeldet. Entitätsverknüpfungen zeigen Pfeile zu ihren Zielen, wenn die Anzeige von Verknüpfungen eingeschaltet ist.

**Aktion** wählt, was die Aktivierung bewirkt: **Abspielen** startet einen benannten Clip oder die ausgewählte Animation; **Umschalten** wechselt zwischen `open` und `close`, wenn es sie gibt, und startet sonst die ausgewählte Animation neu; **Öffnen** und **Schließen** starten diese Clips; **Pausieren** hält die Pose, **Fortsetzen** setzt fort und **Stoppen** stellt die ursprüngliche Pose wieder her. **Nur einmal** überschreibt die Vorgabe der Klasse. **Wiederholungsabstand (s)** legt die kürzeste Zeit zwischen zwei Aktivierungen fest; die erste wird dadurch nicht verzögert.

**Auslöser testen** führt die Aktion in der Vorschau aus, ohne die Map zu bearbeiten oder Rückgängig-Schritte hinzuzufügen. **Vorschau zurücksetzen** stellt die ursprünglichen Posen wieder her und setzt den Einmal- und den Abklingstatus zurück. Kollisionen mit dem Spieler oder Spielskripte werden dabei nicht simuliert.

## Animation in Exporten
Godot exportiert native AnimationPlayer- und AnimationLibrary-Ressourcen, einschließlich Kurven und bewegter Kollision. Ein CharacterBody3D, der den Bereich eines Schalters betritt, aktiviert dessen Ziel. Änderungen an diesen Clips in Godot gelangen nicht zurück in die Map und werden beim nächsten Export ersetzt.

Der Raylib-Export fügt `<map>_animation.h` mit einem Player in C und `<map>_preview.c` mit einer Vorschau mit freier Kamera hinzu; folgen Sie `<map>_raylib.md`, um es zu bauen. In der Vorschau pausiert `SPACE`, `R` startet die Standard-Clips neu und `N` wechselt reihum durch die benannten Clips. Defold bettet die Wiedergabe in das Entity-Skript ein, mit dokumentierten Nachrichten `heron_*_animation`. Build Level in Unity fügt `HeronBrushAnimation` und einen kinematischen Körper hinzu. Der Bevy-Export enthält `heron_animation.rs` und eine Anleitung. Godot, Unity, Defold, Bevy und raylib behalten die Trigger-Verknüpfungen, das Einmal-Flag und den Wiederholungsabstand. Ihr Spiel muss den Spieler trotzdem selbst registrieren und seine eigenen Schalter mit der API der jeweiligen Laufzeitumgebung verbinden.

## Das Beispiel Motion lab
Öffnen Sie `motion_lab` unter Beispiele auf dem Startbildschirm. Es enthält eine Plattform auf einem L-förmigen Pfad, einen Aufzug, eine Drehplattform und eine Tür mit Scharnier samt Schaltern. Öffnen Sie die daneben liegende `godot/project.godot` in Godot und drücken Sie `F5`. WASD und die Maus bewegen den Spieler, Leertaste springt, 1 bis 4 besuchen die Stationen und R startet die aktuelle Station neu. Treten Sie auf eine farbige Platte, um ihr Ziel zu aktivieren. Die beiliegende README erklärt das erneute Exportieren.

# Export in eine Engine
Datei → Exportieren öffnet das Exportfenster. Links sind die Engines mit ihren Logos aufgelistet: Godot, Defold, three.js, Babylon.js, PlayCanvas, Bevy, raylib, Unity, Unreal Engine, Flax Engine, Cocos Creator, Wavefront .obj, glTF .gltf, GLB .glb und zuletzt `.map`. Die Seite rechts sagt, was das Ziel erhält. **Schreiben** schreibt den Export, **Prüfen** baut das Level, ohne es zu schreiben, und listet die Probleme auf, und **Schließen** schließt das Fenster.

Exportieren ist vom Speichern getrennt: Die `.map` bleibt Ihre bearbeitbare Quelle. Jedes Bedienelement speichert die Einstellungen seiner Engine, sobald es sich ändert; so behält jede Engine ihre eigenen Antworten, und das Fenster öffnet sich so, wie Sie es verlassen haben.

## Gemeinsame Optionen
- **Skalierung**: **Einheiten je Meter**, standardmäßig 32.
- **Was geschrieben wird**: **Kollisionsgeometrie**.
- **Optimierungen**: **Statische Meshes in 1024-Einheiten-Zellen gruppieren**, **Flächen im Inneren der Geometrie verwerfen**, **Flächen in derselben Ebene zusammenfassen** und **Kollisionsformen zusammenfassen**.
- **Projektpfade**: wohin das Level und seine Ressourcen innerhalb des Projekts kommen.
- **Werkzeugmaterialien**: die Namen der Materialien clip, skip und origin.
- **Mehr**: engine-spezifische Schalter wie **Knoten benennen nach**, **Ebenen und Gruppen als übergeordnete Knoten wiederherstellen**, **Dreiecksumlauf umkehren**, das Spiegeln von Texturkoordinaten und ein zweiter UV-Satz für gebackenes Licht mit **Lightmap-Texel**.
- **Mit der Karte aktuell halten**: **Bei jedem Speichern der Karte erneut schreiben**. Diese Option steht zur Verfügung, sobald die Engine einmal exportiert wurde, denn dieser Export legt fest, wohin geschrieben wird.

Statische Zellen ordnen Welt-Brushes nach ihren Mittelpunkten zu und fassen sie innerhalb jeder 1024-Einheiten-Zelle nach Material zusammen, damit Engines kleinere Teile der Map per Culling ausblenden können. Die Kollision bleibt erhalten, und Gameplay-Entitäten bleiben getrennt. Das kann mehr Draw Calls bedeuten als ein einziges Welt-Mesh, deshalb ist es standardmäßig ausgeschaltet. Eigenständige OBJ-, glTF- und GLB-Exporte verwenden keine Zellen.

## Hinweise zu den Zielen
- Godot: `res://`-Pfade für Texturen und Materialien, die Physik-Ebene für Trigger und Lightmap-UV2. Wählen Sie einen Exportordner innerhalb des Godot-Projekts.
- Defold: das Verzeichnis für Asset-Ressourcen, das Modellmaterial und der Textur-Sampler, der im gewählten Material vorhanden sein muss.
- Unity: OBJ-Meshes und ein Editor-Importer; exportieren Sie in den Assets-Ordner des Projekts.
- Unreal Engine: ein generiertes Importskript und ein Content-Pfad, normalerweise /Game/<level>.
- Flax Engine: ein Content-Pfad für Modelle relativ zum Content-Ordner des Projekts.
- three.js, Babylon.js, PlayCanvas, Bevy, raylib und Cocos Creator: Szenen oder Code für diese Laufzeitumgebungen, mit einer Anleitung neben dem Export.
- glTF / GLB: portable Geometrie für kompatible Programme; GLB ist die Binärform. OBJ: Geometrie und Materialien für Modellierprogramme.
- `.map`: **Schreiben als** wählt Valve 220, Standard, Quake 2 oder Quake 2 (Valve). Das Fenster sagt, was im gewählten Dialekt verloren ginge.

Lesen Sie nach dem Schreiben die Warnungen, etwa zu fehlenden Texturen. **Prüfergebnisse** listet sie im Fenster auf.

# Live-Verbindungen: Godot, Unity, Defold und Flax
Eine Live-Verbindung überträgt ungespeicherte Änderungen aus Heron Brush während der Arbeit in einen Engine-Editor. Sie funktioniert nur in eine Richtung: Änderungen in der Engine kommen nicht zurück. Pro Map-Fenster kann eine Live-Verbindung laufen. Beenden Sie sie, bevor Sie die Engine wechseln, und starten Sie sie neu, damit geänderte Exportoptionen wirksam werden. Die Bedienelemente befinden sich im Exportfenster auf der Seite der Engine, auf der Karte **Live-Verbindung mit …**.

Die Anzeige in der Titelleiste zeigt den Zustand der Verbindung; ein Klick darauf beendet sie. Wird die Map geschlossen oder ersetzt, endet die Übertragung. Nach einem fehlgeschlagenen Import beheben Sie das Problem und starten die Verbindung erneut.

## Godot
1. Exportieren Sie einmal in ein Godot-Projekt.
2. Klicken Sie auf **Godot-Erweiterung installieren**.
3. Aktivieren Sie Heron Brush Live in Godot unter Projekteinstellungen → Plugins.
4. Klicken Sie auf **Live-Verbindung starten** und hängen Sie die Map über das Godot-Dock an eine geöffnete 3D-Szene an.

Die Erweiterung ersetzt nur ihren generierten Zweig und behält die Platzierung des Anhangs und andere Knoten bei. Speichern Sie die Godot-Szene selbst, und halten Sie Ihre eigenen Skripte und Knoten außerhalb des generierten Zweigs.

## Unity
1. Exportieren Sie einmal in den Assets-Ordner des Unity-Projekts.
2. Klicken Sie auf **Unity-Erweiterung installieren** und lassen Sie Unity sie kompilieren.
3. Öffnen Sie in Unity Window → Heron Brush → Live Link.
4. Starten Sie die Verbindung und hängen Sie die Map an die aktive Unity-Szene an.

Lassen Sie das Live-Fenster in Unity geöffnet. Im Play Mode und im Prefab Mode werden Aktualisierungen pausiert. Nur das Kindobjekt Generated des Map-Objekts wird ersetzt. Bearbeiten Sie die generierten Dateien nicht.

## Defold
1. Exportieren Sie einmal in Ihr Defold-Projekt und lassen Sie das Projekt in Defold geöffnet.
2. Klicken Sie auf **Live-Verbindung starten**. Eine Erweiterung ist nicht nötig.
3. Öffnen Sie heron_brush_live/preview.collection in Defold oder referenzieren Sie es aus Ihrer eigenen Szene.

Die Verbindung schreibt in einen separaten generierten Ordner und lässt den endgültigen Export und game.project unangetastet. **Laufendes Spiel neu laden** aktualisiert auch ein aus Defold gestartetes Spiel; wenn Objekte hinzukommen oder sich die Kollision ändert, muss das Level unter Umständen neu gestartet werden. Fehlgeschlagene Verbindungen werden automatisch erneut versucht.

## Flax
Exportieren Sie in den Content-Ordner des Projekts und klicken Sie auf **Flax-Addon installieren**. Lassen Sie Flax es kompilieren, starten Sie die Verbindung und wählen Sie dann in der geöffneten Flax-Szene Tools → Heron Brush: attach live map. Der generierte Wurzelknoten behält seine Platzierung, andere Objekte bleiben unberührt. Speichern Sie die Szene selbst. Im Play Mode werden Aktualisierungen pausiert. Tools → Heron Brush: stop live map pausiert den Empfang.

# Gemeinsame Live-Sitzungen
Zusammenarbeit → Live-Sitzung öffnet ein Fenster, in dem Sie eine Sitzung für andere Heron-Brush-Nutzer hosten oder einer beitreten. So bearbeiten mehrere Personen gemeinsam eine Map; das ist etwas anderes als eine Live-Verbindung zu einer Engine.

## Hosten und beitreten
Geben Sie **Dein Name** ein. Zum Hosten wählen Sie **Sitzung hosten**, klicken auf **Sitzung starten** und schicken Ihren Gästen die Angaben aus **Verbindungsdaten kopieren**. Zum Beitreten wählen Sie **Einer Sitzung beitreten**, fügen die Angaben ein und klicken auf **Beitreten**. Beim Beitreten wird die geöffnete Map durch die Map des Hosts ersetzt, speichern Sie Ihre Arbeit also vorher.

Der Standard-Build funktioniert im lokalen Netzwerk, und **Port im lokalen Netzwerk** legt den Port fest. Sitzungen über das Internet (**Über das Internet verbinden**) erfordern einen Build mit der optionalen Netzwerkfunktion. Die Teilnehmenden sehen in der 3D-Ansicht die Kameramarkierungen und Namen der anderen.

Wenn Sie das Fenster schließen, läuft die Sitzung weiter; die Anzeige in der Titelleiste zeigt ihren Zustand. **Sitzung beenden** oder **Sitzung verlassen** beendet sie. Bearbeiten zwei Personen dasselbe Objekt, entscheidet die Reihenfolge beim Host über die endgültige Fassung, und auch Rückgängig ist eine gemeinsame Änderung. Arbeiten Sie nach Möglichkeit in getrennten Bereichen und speichern Sie Zwischenstände.

# 3D-Darstellung und ein zweiter Monitor
## Darstellungsmodi
Der Abschnitt **Ansicht** im Tab Map schaltet um, was die Ansichten zeichnen: Brushes, Punktentitäten, Entity-Modelle, Entity-Klassennamen, Gruppenumrisse, Patches, Materialien, Ausgelassene Flächen anzeigen, Animationsvorschau, Drahtgitter, Kanten und Umrisse von Brush-Entitäten. Drahtgitter zeichnet die Kanten der Dreiecke, einschließlich Diagonalen und Kanten hinter Oberflächen, ohne Materialien. Es ist eine vorübergehende Debug-Ansicht und wirkt sich weder auf Exporte noch auf Renderings aus.

Beleuchtung, Schatten, Umgebungsverdeckung, Reflexionen, Bloom und Glättung für die 3D-Ansicht finden Sie unter Einstellungen → Renderer. Das sind Vorschaueinstellungen; fertige Bilder verwenden die eigenen Einstellungen der Renderkamera.

## Ein separates 3D-Fenster
Ansicht → 3D-Ansicht in separatem Fenster verschiebt die 3D-Ansicht in ein eigenes Fenster, das Sie auf einen anderen Monitor legen können. Es teilt Kamera, Dokument und Werkzeuge, und die flachen Ansichten nehmen den Platz im Hauptfenster ein. Sein Menü bietet Materialien, Flächenansicht und Drahtgitter; ein Untermenü Ansicht mit Ausgelassene Flächen anzeigen, Animationsvorschau, Kanten, Gruppenumrisse, Punktentitäten, Entity-Modelle und Umrisse von Brush-Entitäten; Auf Auswahl fokussieren; Vorne, Seite und Oben, die die Kamera um die Auswahl (oder einen Punkt vor der Kamera) drehen; Kamera aus der 3D-Ansicht platzieren; Vollbild (`F11`); Bild rendern; und Ins Hauptfenster zurückkehren. Wenn Sie das Fenster schließen, wird die Ansicht wieder angedockt.

## Prozeduraler Himmel
Der prozedurale Himmel ist in der 3D-Ansicht standardmäßig eingeschaltet und lässt sich unter Einstellungen → Renderer → Licht ausschalten. Renderkameras haben einen eigenen Schalter **Prozeduraler Himmel**, der mit der Map gespeichert wird. Die Farben von Himmel, Horizont und Boden stammen aus den Umgebungseinstellungen, und die Sonne folgt der Szenenbeleuchtung. Der Himmel hat keine Geometrie und wird nicht exportiert.

# Renderkameras und PNG-Bilder
Datei → Bild rendern öffnet das Renderfenster: links die Kamera und ihre Einstellungen, rechts eine Live-Vorschau, die sich bei jeder Änderung neu zeichnet. Kameras werden in der `.map` in einer nur im Editor vorhandenen Ebene Render cameras gespeichert, die von Engine-Exporten ausgenommen ist.

## Die Kamera platzieren
Legen Sie den Bildausschnitt in der 3D-Ansicht fest und drücken Sie `Ctrl+Alt+0` (oder `Ctrl+Alt+Num0`), oder klicken Sie auf **Kamera aus der 3D-Ansicht platzieren**. Ist keine Kamera ausgewählt, wird die erste gespeicherte aktualisiert; gibt es keine, wird eine erstellt. Das lässt sich rückgängig machen und behält die Ausgabe- und Beleuchtungseinstellungen bei. **Kamera in der Karte auswählen**, **Kamera aus der Karte laden** und **3D-Ansicht zur Kamera bewegen** arbeiten in die umgekehrte Richtung. **Kamerarahmen in der 3D-Ansicht** zeigt die Grenze des Ausgabebilds, und **Kamera aus der 3D-Ansicht steuern** lässt die normale Navigation die gespeicherte Kamera bewegen.

## Einstellungen
- **Kameraposition**: X, Y, Z, Gier- und Nickwinkel.
- **Ausgabe und Objektiv**: **Voreinstellungen** (Full HD, Quadrat, Hochformat), Breite, Höhe, Samples, **Vertikales Sichtfeld** und **Ferne Clipping-Ebene**.
- **Beleuchtung**: Tageslicht, Sonnenuntergang oder Bewölkt, oder Belichtung, Sonne sowie Himmels-, Horizont- und Bodenfarbe selbst einstellen, **Prozeduraler Himmel**, **Schatten**, **Umgebungsverdeckung** und **Bloom**.
- **Schärfentiefe**: **Schärfentiefe aktivieren**, Fokusdistanz, Schärfebereich und Unschärferadius. **Auf Auswahl fokussieren** stellt auf den Mittelpunkt eines ausgewählten Objekts vor der Kamera scharf.

**Kamera und Einstellungen speichern** legt das Formular in der Map ab; speichern Sie die Map, damit es auf der Festplatte erhalten bleibt.

## Rendern
**PNG rendern** fragt nach dem Speicherort und startet ein progressives Rendering, das sich in der Vorschau Sample für Sample aufbaut. Mehr Samples sehen besser aus, dauern aber länger. **Rendern abbrechen** hält an, ohne eine Datei zu schreiben. Der Renderer verwendet die Rasterisierungs-Pipeline des Editors, kein Path Tracing. Die PNG-Datei hat einen deckenden Hintergrund, und Animationen werden nicht gerendert. Sehr große Bilder können den Editor kurz anhalten.

# Einstellungen
Datei → Einstellungen (`Ctrl+,`) öffnet die Einstellungen. Änderungen wirken sofort und werden gespeichert. Die Seitenleiste listet Übersicht, Spiele, Ansicht, Farben, Maus, Tastatur und Renderer auf. Das Suchfeld oben findet eine Einstellung auf jeder Seite anhand ihres Namens oder ihrer Beschreibung. Jede geänderte Einstellung hat eine eigene Schaltfläche, die ihren Standardwert wiederherstellt, und **Standardwerte wiederherstellen** setzt die aktuelle Seite zurück.

## Die Seiten
- **Übersicht**: die gängigen Einstellungen auf einen Blick: Darstellung (Deckkraft der Bedienfelder (%), Farbschema, Aufteilung, Sprache), Bearbeiten, Ansicht und Handbuch.
- **Spiele**: das Spiel, wo seine Dateien liegen, und Ihr Name für die Zusammenarbeit. Ein Spiel legt fest, welches Map-Format geschrieben wird, welche Entity-Definitionen geladen werden und wo seine Materialien liegen. Spielkonfigurationen werden aus dem Ordner `games` im Konfigurationsordner (`~/.config/heron-brush/games` unter Linux und macOS, `%LOCALAPPDATA%\heron-brush\games` unter Windows) und aus dem Ordner `games` neben der Anwendung gelesen, in den die Pakete die mitgelieferte Konfiguration legen. Fehlt im Spielordner der Ordner, den die Spielkonfiguration nennt, etwa `baseq3` von Quake 3 in einer OpenArena-Installation, verwendet Heron einen Ersatz, den die Konfiguration kennt (`baseoa`), sonst den einzigen Ordner dort mit `.pak`- oder `.pk3`-Paketen, und sagt das unter **Mods**. In **Mods** genannte Ordner werden weiterhin darüber eingebunden.
- **Ansicht**: Aufteilung, Farbschema, Sprache, Weltachsen, Deckkraft des Rasters, Zellengröße für die Leck-Prüfung und die Mapping-Sperren.
- **Farben**: die Farben der Ansichten, der Geometrie und der Anfasser.
- **Maus**: Geschwindigkeiten und Invertierungen für Umsehen, Verschieben und Bewegen, Fluggeschwindigkeit, Sichtfeld, Die 2D-Ansichten koppeln und Größe der Anfasser.
- **Tastatur**: die Tastenbelegung, Heron Brush oder Blender.
- **Renderer**: realistische Schattierung, Auswahlumriss, Licht, Schatten, Umgebungsverdeckung, Reflexionen, Bloom und Oberflächen.

## Sprache
**Sprache** bietet Der Arbeitsumgebung folgen und jede übersetzte Sprache. Die Oberfläche wechselt sofort. Das Handbuch öffnet sich in der Sprache der Oberfläche, wenn es eine Übersetzung davon gibt, und sonst auf Englisch.

# Plugins und MCP
## Plugins
Plugins erweitern den Editor mit Rhai-Skripten. Datei → Plugins öffnet das Fenster Plugins. **Plugin installieren** installiert eine `.rhai`-Datei und **Plugin-Ordner installieren** einen Ordner, der `main.rhai` enthält; optionale Metadaten in `plugin.toml` beschreiben das Plugin und seine Einstellungen. Plugins sind unter **Aktive Plugins** und **Inaktive Plugins** gruppiert, und **Plugin aktivieren** schaltet eines ein oder aus; das bleibt über einen Neustart hinweg erhalten. **Plugins neu laden** liest die Skripte erneut von der Festplatte und setzt die Werte in ihren Bedienfeldern zurück.

Aktive Plugins erscheinen außerdem im Abschnitt **Plugins** des Tabs Map, mit ihren Einstellungen, Aktionsschaltflächen (**Ausführen**) und ihrer Ausgabe. Eine Aktion wirkt auf die Auswahl oder, wenn nichts ausgewählt ist, auf alles und ist ein einziger Rückgängig-Schritt. Das mitgelieferte Plugin Selection inspector zeigt Anzahlen und Aktionen zum Nach-oben- und Nach-unten-Verschieben.

## MCP
Zusammenarbeit → Für einen Agenten bereitstellen (MCP) lässt einen KI-Agenten oder ein anderes Werkzeug den geöffneten Editor über das Model Context Protocol steuern. Das Fenster zeigt die lokale Adresse mit **Kopieren**, und **Server stoppen** beendet den Server; solange er läuft, zeigt die Titelleiste **MCP aktiv**. Die Kommandozeilenoption `--mcp` stellt dieselbe Schnittstelle ohne Fenster bereit. Clients können Werkzeuge für Geometrie, Assets, Eigenschaften, Animation, Export und Sitzungen abfragen. MCP und Editor-Plugins sind unterschiedliche Schnittstellen.

# Kommandozeile
Das Programm heißt `heron-brush`. Ohne Optionen öffnet es den Startbildschirm.

## Dateien öffnen
`heron-brush map.map [entities.fgd] [materials/]` öffnet eine Map, optional mit Entity-Definitionen und einem Materialordner. Ein leeres Argument überspringt eines davon, zum Beispiel `heron-brush map.map "" materials/`. Läuft bereits ein Editor, öffnet sich die Map darin; `--new-instance` öffnet stattdessen ein separates Fenster.

## Befehle
- `--help` oder `-h`: die Verwendung ausgeben. `--version` oder `-V`: die Version ausgeben.
- `--mcp`: MCP über Standardeingabe und -ausgabe bereitstellen, ohne Fenster.
- `--merge-map <base> <ours> <theirs> [<output>]`: ein Drei-Wege-Merge von `.map`-Dateien für git. Die Ausgabe ist standardmäßig ours. Konflikte werden ausgegeben und führen zu einem Exit-Code ungleich null.
- `--obj-to-map <in.obj> <out.map> [size] [thickness] [snap] [material]`: ein Mesh in Brushes umwandeln. size ist die Länge der längsten Seite in Map-Einheiten (Standard 256), snap ist ein Raster für die Ecken (0 für keines), und die Wörter `solid` oder `layered` erzeugen Rasterquader bzw. konvexe Platten statt dünner Keile.
- `--render-preview <in.map> <out.png> [size]`: ein PNG-Vorschaubild einer Map schreiben, standardmäßig 512 Pixel.
- `--run-script <script.rhai> <in.map> [out.map] [key=value ...]`: ein Plugin-Skript mit allem ausgewählt auf eine Map anwenden und das Ergebnis schreiben (standardmäßig zurück in die Eingabedatei). `key=value` beantwortet die Einstellungen des Plugins.

# Fehlerbehebung
## Wenn etwas falsch aussieht
- Fehlendes Material: Prüfen Sie den Materialordner oder das Paket und die Projektpfade des Exports.
- Falsche Modellgröße: Vergleichen Sie mit der 64 Einheiten hohen Spielermarkierung und dem Exportmaßstab.
- Falsche Verlaufsrichtung: Prüfen Sie den Verlaufswinkel und die Ausrichtung der Fläche.
- Leere Szene in der Engine: Lesen Sie die Exportwarnungen und folgen Sie den Importanweisungen des Ziels.
- Live-Verbindung wartet: Prüfen Sie, ob die Erweiterung installiert, die Szene angehängt und die Aktualisierung nicht pausiert ist.
- Nur Linien, keine Flächen: Schalten Sie Drahtgitter im Abschnitt Ansicht des Tabs Map aus, oder wählen Sie Materialien oder Flächenansicht im Menü des 3D-Fensters.
- Objekte fehlen in den Ansichten: `Ctrl+Shift+I` zeigt ausgeblendete Objekte; prüfen Sie die Ebenen und die Ansichtsfilter.
- Eine Taste bewirkt nichts: Klicken Sie zuerst in die Ansicht, damit sie den Tastaturfokus hat, und prüfen Sie die Tastenbelegung unter Einstellungen → Tastatur.
- Eine Bearbeitung ist fehlgeschlagen: Machen Sie sie rückgängig, vereinfachen Sie die Geometrie und verwenden Sie **Map prüfen** und **Auf Lecks prüfen** im Abschnitt **Probleme** des Tabs Map.
- Eine Map lässt sich nicht aus dem Dateimanager öffnen: Sie wurde womöglich an einen bereits laufenden Editor übergeben; suchen Sie nach diesem Fenster.

## Einstellungen und Sicherungen
Einstellungen, zuletzt verwendete Maps, Sterne, Exporteinstellungen und Plugins liegen im Konfigurationsordner: `~/.config/heron-brush` unter Linux und macOS und `%LOCALAPPDATA%\heron-brush` unter Windows. Automatische Sicherungen liegen im Ordner `autosave` neben der jeweiligen Map.

## Einen Fehler melden
Geben Sie die Version des Editors (Hilfe → Über), Ihr Betriebssystem, eine kleine Map, die das Problem zeigt, und die genauen Schritte an. Der Ausgabebereich (`Ctrl+4`) verrät oft, was schiefgelaufen ist. Speichern Sie eine Kopie, bevor Sie experimentieren.

## Dieses Handbuch verwenden
Wählen Sie links im Inhaltsverzeichnis ein Kapitel. `Ctrl+F` zeigt das Inhaltsverzeichnis an und springt in das Suchfeld; `Esc` leert die Suche oder schließt das Handbuch, wenn die Suche leer ist. **Inhalt** blendet die Kapitelliste aus oder ein, und **Schriftgröße** ändert die Schriftgröße; beides wird gespeichert. Die Sprachliste oben wechselt zwischen den verfügbaren Übersetzungen.
