# Per iniziare
Heron Brush è un editor di livelli basato sui brush. Costruisci i livelli con brush convessi, dipingi e texturizzi le loro facce, posizioni le entità, animi porte e piattaforme ed esporti il risultato verso un motore di gioco. Offre anche asset integrati, forme parametriche, collegamenti live con gli editor dei motori, sessioni di modifica condivise e il rendering di immagini statiche. Questo manuale è incluso nell'applicazione e funziona anche offline.

Su macOS, `Cmd` funziona ovunque questo manuale indichi `Ctrl`, e Option corrisponde ad Alt.

## La schermata di benvenuto
Quando Heron Brush si avvia senza una mappa, si apre la schermata di benvenuto. A sinistra trovi **Continua** (l'ultima mappa che avevi aperto), **Nuova mappa** e **Sfoglia** per aprire un file `.map` esistente, oltre a **Gioco**, **Manuale** e **Preferenze**. A destra c'è l'elenco delle mappe recenti con le miniature, seguito da **Esempi**. La stella (**Fissa in alto**) fissa una mappa in cima all'elenco; il cestino (**Sposta nel cestino**) sposta il file nel cestino di sistema, da cui è possibile ripristinarlo. I tasti freccia permettono di muoversi nella schermata e Invio apre l'elemento attivo. File → Benvenuto riporta la schermata in qualsiasi momento.

## La tua prima mappa
1. Scegli **Nuova mappa** nella schermata di benvenuto, oppure File → Nuova mappa (`Ctrl+N`).
2. Scegli lo strumento Brush (`B`) nella barra degli strumenti e trascina in una vista 2D per disegnare la forma selezionata. Controlla la profondità in un'altra vista 2D.
3. Seleziona il brush e spostalo trascinandolo. Usa la scheda **Faccia** per assegnargli un materiale o un colore.
4. Salva la mappa sorgente con `Ctrl+S`. Quando vuoi usarla in un gioco, apri File → Esporta e scrivi una scena per il tuo motore.

## File e salvataggio
Il file `.map` è la tua sorgente modificabile. Salva spesso: l'esportazione, i collegamenti live e il rendering non salvano mai la mappa al posto tuo. File → Salva come (`Ctrl+Shift+S`) aggiunge l'estensione `.map` se la ometti. File → Torna alla mappa salvata scarta le modifiche non salvate e ricarica il file. Se chiudi una finestra o apri un'altra mappa con modifiche non salvate, Heron Brush chiede se **Salva**, **Scarta** o **Annulla**.

Finché una mappa salvata ha modifiche non salvate, ogni dieci minuti vengono scritti backup numerati in una cartella `autosave` accanto alla mappa. Vengono conservati i 50 backup più recenti.

Se apri una mappa mentre Heron Brush è già in esecuzione, la mappa viene passata alla finestra aperta, che viene portata in primo piano. Usa `--new-instance` dalla riga di comando quando vuoi una finestra separata (vedi Riga di comando).

# L'interfaccia
La finestra principale ha una barra del titolo con i menu, sotto di essa la barra degli strumenti, poi le viste della mappa, l'ispettore a destra e una barra di stato in basso.

## Barra del titolo e menu
La barra del titolo mostra il logo, i menu, il nome della mappa al centro e i pulsanti della finestra. Su macOS i consueti pulsanti rosso, giallo e verde si trovano a sinistra. Trascina la barra del titolo per spostare la finestra e fai doppio clic per massimizzarla. Quando qualcosa è in esecuzione in background, verso la fine della barra compare un indicatore: un collegamento live con un motore, una sessione dal vivo oppure **MCP attivo**. Fai clic su un indicatore per interrompere il collegamento live o per aprirne la finestra.

- **File**: Benvenuto, Nuova mappa (`Ctrl+N`), Apri mappa (`Ctrl+O`), Salva (`Ctrl+S`), Salva come (`Ctrl+Shift+S`), Torna alla mappa salvata, Esporta, Renderizza immagine, Plugin e Preferenze (`Ctrl+,`).
- **Modifica**: Annulla operazione (`Ctrl+Z`), Ripeti (`Ctrl+Shift+Z`), Taglia, Copia, Incolla, Duplica (`Ctrl+D`), Elimina, Seleziona tutto (`Ctrl+A`), Ripeti gli ultimi comandi (`Ctrl+R`) e Dimentica i comandi ripetibili.
- **Vista**: Vista 3D in una finestra separata, Vai a, Aggancia alla griglia (`Alt+0`), Mostra griglia (`0`), Anteprima animazioni e Mostra facce omesse.
- **Geometria**: Inserisci testo 3D, Unisci angolo, Converti le facce in patch, Togli la selezione dalla sua entità e Sincronizza i gruppi collegati.
- **Animazione**: Animazione con fotogrammi chiave, Anteprima animazioni e Posiziona la camera dalla vista 3D (`Ctrl+Alt+0`).
- **Mappa**: Carica il point file, Ricarica il point file e Chiudi il point file; Carica il file dei portali, Ricarica il file dei portali e Chiudi il file dei portali. Mostrano le tracce dei leak e i portali scritti dai compilatori di mappe.
- **Collaborazione**: Sessione dal vivo e Rendi disponibile a un agente (MCP).
- **Aiuto**: Manuale e Informazioni.

## La barra degli strumenti
La barra degli strumenti contiene tutti gli strumenti. Il primo gruppo è quello che si usa continuamente: Seleziona, Brush, Taglio, Vertice, Spigolo, Faccia, Ruota e Scala. Il secondo gruppo contiene Inclina, Assembla, Trascinamento, Punto di controllo, Raccordo e Dipingi. Passa il puntatore su un pulsante per vederne il nome e la scorciatoia. Facendo di nuovo clic sullo strumento attivo si torna a Seleziona.

Dopo gli strumenti vengono **Ribalta orizzontalmente** e **Ribalta verticalmente**, poi due interruttori: **Blocco** mantiene la mappatura del materiale quando un brush si sposta e **UV** la mantiene quando un brush ruota (`U` lo attiva e disattiva). Seguono le impostazioni dello strumento attivo: lo strumento Brush mostra la forma e le sue opzioni, lo strumento Raccordo mostra raggio, facce e angoli con **Applica**. All'estremità destra si trovano il nome dello strumento, la dimensione della griglia (**Griglia 16** e così via; fai clic per sceglierne un'altra) e il menu **Impostazioni griglia** con Aggancia alla griglia, Mostra griglia, Anteprima animazioni, Mostra facce omesse e Animazione con fotogrammi chiave.

## Viste della mappa
Le viste sono quattro: la vista 3D e tre viste piane, Dall'alto (XY), Frontale (XZ) e Laterale (YZ). Preferenze → Vista → Disposizione permette di scegliere tra Un riquadro, Due riquadri, Tre riquadri e Quattro riquadri. Trascina i divisori per ridimensionare i riquadri. Un clic in una vista le assegna la tastiera. `Space` passa alla vista successiva in un riquadro che mostra una vista alla volta, e `Ctrl+Space` massimizza la vista che ha la tastiera e ripristina poi la disposizione.

Fai clic destro senza trascinare in una qualsiasi vista per aprire un menu contestuale relativo alla selezione e a ciò che si trova sotto il puntatore. Offre raggruppamento, livelli, gruppi collegati, creazione di entità, spostamento dei brush dentro o fuori da un'entità, copia del nome di un materiale e la sua visualizzazione nel browser dei materiali.

## L'ispettore
L'ispettore a destra ha quattro schede. Trascinane il bordo sinistro per cambiarne la larghezza.

- **Mappa** (`Ctrl+1`): Livelli, Gruppi, Plugin, Problemi, filtri della sezione Vista e Tag.
- **Entità** (`Ctrl+2`): il browser delle entità, le definizioni delle entità, le proprietà dell'entità selezionata e, per pulsanti e trigger, Attivatore animazione.
- **Faccia** (`Ctrl+3`): UV, Materiali, Colore, Trascinamento, Raccordo, Patch e Rientro e smusso. Ogni sezione si può comprimere.
- **Asset**: la libreria di modelli brush riutilizzabili.

`Ctrl+5` nasconde o mostra l'ispettore e `Ctrl+Alt+T` nasconde o mostra la barra degli strumenti, lasciando più spazio alle viste.

## La barra di stato e il pannello di output
La barra di stato in basso mostra l'ultimo messaggio dell'editor. Gli errori sono in rosso. Fai clic sulla barra, oppure premi `Ctrl+4`, per aprire il pannello di output con la cronologia completa dei messaggi. La maggior parte dei comandi riporta qui cosa ha fatto, per esempio quanti brush sono stati tagliati.

## Finestre di dialogo e aspetto
Le finestre di dialogo come Preferenze, Esporta e Renderizza immagine si aprono in finestre proprie. Preferenze → Vista → Schema di colori permette di scegliere tra Segui il sistema, Chiaro e Scuro. **Opacità dei pannelli (%)** nella pagina Panoramica rende i pannelli trasparenti sopra le viste. La finestra disegna i propri pulsanti nello stile del sistema: nativo su macOS, stile Windows 11 su Windows e stile GNOME su Linux.

# Navigazione e tastiera
## Muoversi nella vista 3D
Tieni premuto il tasto destro del mouse e trascina per guardarti intorno. Trascina con il tasto centrale per fare il pan. La rotellina sposta avanti e indietro. `W`, `A`, `S` e `D` volano in avanti, a sinistra, indietro e a destra, `Q` sale e `X` scende; tieni premuto `Shift` per passi più lunghi. Con **Alt + trascinamento col tasto centrale muove la camera** attivato in Preferenze → Mouse, Alt con il tasto centrale sposta la camera lungo la direzione di vista. Velocità, inversioni e campo visivo si trovano in Preferenze → Mouse.

## Muoversi nelle viste 2D
Trascina con il tasto centrale per fare il pan e usa la rotellina per zoomare attorno al puntatore. Il pan e lo zoom di una vista piana mantengono allineate le altre; lo controlla **Collega le viste 2D** in Preferenze → Mouse. `Ctrl+Alt+Z` reimposta lo zoom e `Ctrl+Shift+U` riporta le viste piane all'origine.

`Ctrl+U` inquadra la selezione in tutte le viste. Vista → Vai a (`Ctrl+Alt+M`) sposta la camera alle coordinate digitate, seleziona ciò che si trova a una riga del file della mappa oppure sposta la selezione della quantità digitata.

## Griglia e aggancio
`+` e `-` cambiano la dimensione della griglia, e i tasti numerici da `1` a `9` scelgono direttamente una dimensione, da 1 a 256 unità. `0` o `#` mostra o nasconde la griglia, e `Alt+0` attiva o disattiva l'aggancio. `S` aggancia i vertici dei brush selezionati alla griglia corrente, `Ctrl+Shift+V` li aggancia a unità intere e `Ctrl+Alt+Shift+V` alla griglia.

## Scorciatoie comuni
- `Ctrl+S`: salva la mappa. `Ctrl+Shift+S`: salva con nome.
- `Ctrl+Z`: annulla. `Ctrl+Shift+Z` o `Ctrl+Y`: ripeti.
- `Ctrl+C`, `Ctrl+X`, `Ctrl+V`: copia, taglia e incolla. Incolla mette gli oggetti al puntatore; `Ctrl+Alt+V` mantiene le loro coordinate originali. Con delle facce selezionate, la copia prende il loro materiale e le impostazioni UV.
- `Ctrl+D`: duplica. `Ctrl+Shift+D`: duplicato collegato di un gruppo.
- `Delete` o `Backspace`: elimina la selezione.
- `Esc`: annulla il trascinamento in corso, torna indietro di un passo nello strumento attivo, torna a Seleziona e infine svuota la selezione. `Shift+Esc` torna direttamente a Seleziona.
- `Ctrl+R`: ripete gli ultimi comandi ripetibili. `Ctrl+Shift+R`: li dimentica.
- `Ctrl+Alt+0` o `Ctrl+Alt+Num0`: posiziona la camera di rendering nel punto della vista 3D.
- `F5`: ricarica i materiali. `F6`: ricarica le definizioni delle entità.

## Scorciatoie di selezione
- `Ctrl+A`: seleziona tutto, oppure tutte le maniglie mentre è attivo uno strumento vertice, spigolo o faccia. `Ctrl+Shift+A`: deseleziona tutto. `Ctrl+Alt+A`: inverte la selezione.
- `Ctrl+B`: seleziona gli altri oggetti della stessa entità o dello stesso gruppo.
- `Ctrl+T`: seleziona tutto ciò che i brush selezionati toccano. `Ctrl+E`: seleziona tutto ciò che si trova al loro interno. `Ctrl+Shift+E`: seleziona tutto ciò che si trova nella loro colonna lungo l'asse di vista. Questi comandi consumano i brush usati per selezionare.
- `Ctrl+I`: isola la selezione. `Ctrl+Alt+I`: la nasconde. `Ctrl+Shift+I`: mostra di nuovo tutto.
- `Ctrl+G`: raggruppa. `Ctrl+Shift+G`: scioglie il gruppo.

## Scorciatoie di trasformazione
- I tasti freccia e `Page Up` / `Page Down` spostano la selezione di un passo di griglia nella direzione che indicano nella vista attiva. Con `Ctrl` duplicano durante lo spostamento, con `Alt` ruotano.
- `Alt+X`, `Alt+Y`, `Alt+Z`: ruotano di un quarto di giro attorno a quell'asse; aggiungi `Shift` per la direzione opposta.
- `Ctrl+F`: ribalta rispetto alla vista; `Ctrl+Alt+F`: ribalta verticalmente. `Ctrl+Shift+X` e `Ctrl+Shift+Y` specchiano lungo X o Y.
- `Insert`: posiziona un'entità puntuale al centro della selezione.
- `Tab`: con un brush selezionato, scorre tra Seleziona → Vertice → Spigolo → Faccia → Seleziona.

## La mappatura dei tasti di Blender
Preferenze → Tastiera → Mappatura dei tasti offre Heron Brush (quella predefinita, usata in tutto questo manuale) e Blender. Con Blender: `G` seleziona e sposta, `R` ruota, `S` scala, `E` esegue il trascinamento, `1`, `2` e `3` scelgono gli strumenti vertice, spigolo e faccia, `K` taglia, `B` seleziona, `A` seleziona tutto e `Alt+A` annulla la selezione, `X` elimina, `Shift+D` duplica, `H` nasconde e `Alt+H` mostra tutto, e `Ctrl+Shift+Z` ripete. Tutto il resto è uguale in entrambe.

# Brush e strumenti di modellazione
I brush sono solidi convessi definiti da piani. Costruisci gli oggetti complessi con più brush semplici invece di cercare di rendere concavo un singolo brush. Mantieni basso il numero di brush dove basta una forma semplice.

## Disegnare i brush
Scegli lo strumento Brush (`B`) e trascina in una vista. La barra degli strumenti mostra **Forma da disegnare**: Parallelepipedo, Scala a gradini, Arco (volta), Cilindro, Cono, Sferoide (UV), Sferoide (icosaedro) e Inserisci testo 3D. Da `Alt+1` ad `Alt+7` scegli direttamente una forma. A seconda della forma, la barra mostra **Svuota** e **Spessore:**, **Altezza del gradino:** e **Direzione:** per le scale, **Asse:**, **Lati:** e la modalità del cerchio per le forme rotonde, **Anelli:** per le sfere e **Precisione:** per l'icosaedro. Imposta le opzioni prima di trascinare. Controlla il risultato in più di una vista.

## Selezionare
Fai clic su un brush per selezionarlo e `Ctrl`-clic per aggiungerlo alla selezione. Un trascinamento da uno spazio vuoto disegna un riquadro di selezione. Nella vista 3D, `Shift`-clic seleziona una singola faccia e `Ctrl+Shift`-clic aggiunge altre facce. Trascinando un brush selezionato lo sposti; trascinandone uno non selezionato selezioni e sposti solo quello.

Nella vista 3D, trascina con `Shift` una faccia di un brush selezionato per estruderla. `Alt`-clic su un'altra faccia copia su di essa il materiale e l'allineamento della faccia selezionata; `Alt+Ctrl` copia solo il materiale e `Alt+Shift` copia la rotazione.

## Strumenti di modellazione
- **Seleziona** (`Esc`): seleziona e sposta.
- **Vertice** (`V`), **Spigolo** (`E`), **Faccia** (`F`): trascina vertici, spigoli o facce. Trascinando con `Shift` dal centro di uno spigolo o di una faccia vi aggiungi un vertice. `Delete` rimuove i vertici scelti.
- **Ruota** (`R`), **Scala** (`T`), **Inclina** (`G`): trasformano la selezione tramite maniglie.
- **Taglio** (`C`): fai clic su due o tre punti per definire un piano di taglio. `Enter` taglia, `Ctrl+Enter` sceglie quale parte tenere e `Backspace` o `Esc` rimuove l'ultimo punto.
- **Assembla** (`N`): posiziona dei punti attorno a un volume e premi `Enter` per crearne un brush convesso. `Backspace` rimuove l'ultimo punto.
- **Trascinamento** (`Y`): vedi la sezione Trascinamento più avanti.
- **Punto di controllo** (`P`): rimodella la griglia di controllo di una patch.
- **Raccordo** (`O`): fai clic su uno spigolo per arrotondarlo, oppure scegli spigoli o vertici con i rispettivi strumenti e premi `Enter`. Più facce danno un risultato più morbido e più geometria.
- **Dipingi** (`K`): vedi Materiali e UV.

## Combinare i brush
- `Ctrl+J`: unisce i brush selezionati nel loro inviluppo convesso.
- `Ctrl+K`: sottrae i brush selezionati dai brush che toccano.
- `Ctrl+Shift+K`: svuota la selezione, con pareti spesse un passo di griglia.
- `Ctrl+L`: interseca la selezione.

## Trascinamento, raccordo, patch e smusso
**Trascinamento** crea geometria lungo un percorso a partire dalle facce selezionate. Seleziona le facce (nello strumento Trascinamento, con `Alt`-clic), poi imposta **Spostamento**, **Rotazione** (asse e angolo), **Scala**, **Passi** e **Percorso** (allineamento Intero o Libero; Arco, Retta o Curva a S) nella sezione Trascinamento della scheda Faccia e premi **Trascinamento**. La riga sotto i controlli indica quanti brush verranno creati.

La sezione **Raccordo** arrotonda gli spigoli scelti con **Raggio**, **Facce** e **Angoli** (Augnato o Arrotondato); con una sola faccia si ottiene uno smusso.

**Patch** mostra **Colonne** e **Righe** della patch selezionata, con pulsanti per aggiungerne o toglierne una coppia. Geometria → Converti le facce in patch (`Ctrl+P`) crea patch dalle facce selezionate.

**Rientro e smusso** agisce sulle facce selezionate: **Rientro** ritira la faccia dai suoi bordi e **Profondità** la spinge in fuori (o in dentro, se negativa). **Smussa le facce** applica entrambi; il terzo pulsante aggiunge un pannello o scava una rientranza, a seconda del segno di Profondità. **Unisci angolo** apre una finestra di dialogo che unisce due brush con un angolo augnato o arrotondato, con anteprima.

## Testo 3D
Geometria → Inserisci testo 3D (anche l'ultima voce dell'elenco delle forme dello strumento Brush) crea brush a partire da un testo. Usa Heron Block, un font poligonale integrato con angoli smussati, larghezze proporzionali e accenti cechi; le lettere minuscole usano le forme maiuscole. Digita il **Testo**, imposta **Altezza** (l'altezza delle maiuscole; gli accenti la superano) e **Profondità**, osserva l'anteprima dal vivo e premi **Inserisci**.

Se un'operazione non riesce a produrre una geometria valida, regola le sue dimensioni oppure annulla e semplifica la forma.

## Serie
Geometria → Serie copia la selezione e mostra le copie nella mappa mentre cambi i valori. I numeri includono l'originale e una serie crea al massimo 1024 copie.

- **Fila** sposta ogni copia rispetto alla precedente di **Passo X**, **Passo Y** e **Passo Z** e la ruota di **Rotazione per copia (gradi)** attorno all'asse scelto.
- **Griglia** riempie **Numero X**, **Numero Y** e **Numero Z** elementi distanziati di **Passo X**, **Passo Y** e **Passo Z**.
- **Anello** distribuisce gli elementi su **Angolo (gradi)** attorno a un centro a **Raggio (unità)** dalla selezione. Con **Rivolgi al centro** ogni copia ruota insieme all'anello.
- **Dispersione** mette le copie in punti casuali entro il raggio, trasversalmente all'asse scelto. Lo stesso **Seme** dà la stessa disposizione e **Rotazione casuale** ruota ogni copia.

Quando la selezione è un gruppo intero, **Copie collegate** rende ogni copia un duplicato collegato, quindi una modifica a una raggiunge tutte. **Applica** mantiene le copie e le seleziona insieme all'originale; **Annulla** le rimuove. L'intera serie si annulla in un solo passaggio.

## Scolpire il terreno
Il terreno è fatto di molti piccoli brush la cui parte superiore il pennello di scultura alza e abbassa. Seleziona un parallelepipedo piatto e scegli Geometria → Dividi in colonne: viene tagliato in colonne triangolari larghe un passo di griglia, messe in un gruppo chiamato Terrain. La parte superiore di un triangolo è sempre piana, quindi le colonne possono formare creste e valli.

Scegli lo strumento **Scolpisci** (`J`) e trascina sul terreno nella vista 3D. L'anello arancione mostra il pennello. Senza selezione il pennello cambia ogni brush sotto di sé; con una selezione, solo quelli selezionati. La barra in cima alla finestra sceglie cosa fa un tratto:

- **Alza** e **Abbassa** spostano la superficie in su o in giù, di più al centro dell'anello e per niente sul bordo. Tieni premuto `Shift` per scambiarli.
- **Leviga** spiana le gobbe.
- **Appiattisci** porta la superficie all'altezza dove è iniziato il tratto.

I due numeri accanto alle modalità sono **Raggio (unità)** e **Forza (%)**. Si muovono solo gli angoli superiori, in verticale; la base resta dov'è e una colonna resta spessa almeno un'unità. Un tratto si annulla in un solo passaggio. Nelle viste 2D lo strumento seleziona come lo strumento Seleziona.

# Materiali e UV
## Caricare e applicare i materiali
Apri la sezione **Materiali** della scheda Faccia. **Carica cartella** aggiunge una cartella di immagini e **Carica .wad** aggiunge un pacchetto di texture; entrambi compaiono sotto **Collezioni**, dove la croce ne rimuove uno. Digita in **Filtra i materiali** per restringere l'elenco. Fai clic su un materiale per renderlo corrente, poi **Applica alla selezione** lo assegna a tutte le facce selezionate. **Seleziona** offre Seleziona i brush che lo usano e Seleziona le facce che lo usano. **Sostituisci** scambia un materiale con un altro in tutta la mappa o solo nella selezione.

Lo strumento **Dipingi** (`K`) applica il materiale corrente alla faccia sotto il puntatore: un clic dipinge una faccia, un doppio clic dipinge l'intero brush e `Shift`-clic preleva il materiale della faccia come materiale corrente, come un contagocce.

## L'editor UV
La sezione **UV** della scheda Faccia mostra il materiale e il contorno della faccia selezionata. Controlli del mouse sulla tela:
- La rotellina zooma attorno al puntatore. Trascina con il tasto destro o centrale per fare il pan. Fai doppio clic con il tasto destro per inquadrare di nuovo la faccia; selezionando un'altra faccia, questa viene inquadrata automaticamente.
- Il trascinamento con il tasto sinistro sposta il materiale mentre il contorno della faccia resta fermo. L'anello ruota la mappatura, le maniglie agli angoli la scalano e la maniglia centrale imposta l'origine di queste modifiche.
- `Alt` più trascinamento sinistro sul bordo di una tessera applica un'inclinazione, come anche `Shift` più trascinamento centrale.
- Ogni trascinamento è un singolo passo di annullamento, e `Esc` annulla un trascinamento in corso.

Sotto la tela si trovano **Scostamento**, **Scala**, **Angolo** e, se il gioco li usa, **Flag di contenuto**, **Flag di superficie** e **Valore di superficie**. I pulsanti sono **Azzera**, **Allinea al mondo**, **Adatta**, **Ribalta H**, **Ribalta V** e le due rotazioni di 90°. **Allinea** orienta il materiale lungo uno degli spigoli della faccia (premi di nuovo per lo spigolo successivo, con `Shift` per quello precedente). I quattro pulsanti di giustificazione spingono il materiale contro un bordo. **Adatta U** e **Adatta V** scalano in modo che un numero intero di ripetizioni copra la faccia; premi di nuovo per il numero successivo e tieni premuto `Ctrl` per mantenere la suddivisione di un trim sheet.

## Tasti UV
Con delle facce selezionate, i tasti freccia spostano il materiale, `[` e `]` lo ruotano di 15°, `,` e `.` ne dimezzano o raddoppiano la scala, `Ctrl+F` e `Ctrl+Alt+F` lo ribaltano, `Shift+R` lo azzera, `Alt+Shift+R` lo riallinea agli assi del mondo e `Ctrl+Shift+F` lo adatta a ciascuna faccia.

**Blocco** nella barra degli strumenti mantiene la mappatura quando i brush si spostano, e **UV** (`U`) la mantiene quando i brush ruotano.

## Materiali degli strumenti
- skip: rimuove la faccia marcata dalle esportazioni verso i motori.
- clip: crea una collisione senza geometria visibile.
- origin: indica il perno di un'entità brush.

I nomi si possono cambiare nella scheda **Materiali degli strumenti** della finestra Esporta. Usa skip per le facce che sai essere nascoste.

## Omettere facce dall'esportazione
Seleziona una faccia e attiva **Ometti faccia durante esportazione** nella sezione UV. Un tratteggio arancione la evidenzia nell'anteprima UV. Il suo materiale e la sua pittura restano salvati, e il brush rimane chiuso e modificabile. Le mesh di rendering esportate tralasciano quella faccia, mentre la collisione viene mantenuta. Lasciare un solo quad di un cubo produce due triangoli di rendering. Questo non rende le facce a doppia faccia e non unisce le mesh.

**Ometti facce coperte** marca le facce completamente coperte da brush statici, comprese le facce identiche a contatto e i brush racchiusi. Agisce sui brush selezionati oppure, se non è selezionato nulla, su tutta la geometria del mondo visibile e non bloccata. Le entità brush, i materiali trasparenti e i brush che hanno già facce omesse vengono saltati. Le facce coperte solo in parte non vengono divise. Un solo annullamento ripristina tutto. Vista → Mostra facce omesse nasconde o mostra queste facce nelle viste dell'editor; la selezione con il clic usa comunque l'intero brush.

# Colore e sfumature
La sezione **Colore** della scheda Faccia dipinge le facce con colori pieni o sfumature sopra i loro materiali. È utile per il blockout di un livello e per uno stile grafico stilizzato. Le facce dipinte mantengono i loro colori reali nelle viste; la selezione è indicata dal contorno.

## Tavolozza e colore di miscela
Fai clic su uno dei dodici campioni della tavolozza per dipingere con esso le facce selezionate, o interi brush selezionati. **Opacità** imposta l'opacità della prossima pittura, in percentuale. **Mescola** ha un pulsante colore che apre il selettore di colore; trascina nel selettore e la selezione si aggiorna dal vivo, oppure premi **Dipingi** per applicare il colore di miscela in un solo passo. **Torna al materiale** rimuove la pittura, così che la faccia mostri di nuovo il suo materiale. La riga in cima alla sezione indica cosa è selezionato e cosa ha applicato.

## Il selettore di colore
Tutti i pulsanti colore dell'editor aprono lo stesso selettore. Trascina nel quadrato per scegliere saturazione e luminosità, e nella striscia accanto per scegliere la tonalità. Sotto ci sono i valori **R**, **G** e **B** e un cursore **Opacità** dove il colore la prevede. Il campo esadecimale accetta `#RRGGBB` o `#RRGGBBAA`; diventa rosso finché il testo non è un colore valido. **Copia** copia il codice esadecimale negli appunti. Fai clic fuori dal selettore per chiuderlo.

## Sfumature
**Sfumatura** permette di scegliere tra Gradiente lineare e Gradiente radiale, e il pulsante accanto inverte i colori da un capo all'altro. Ogni punto ha un pulsante colore e **Posizione (%)**, e i punti intermedi hanno un pulsante per rimuoverli. **Aggiungi punto colore** aggiunge un punto nello spazio più ampio; una sfumatura può avere fino a otto punti. **Angolo della sfumatura** imposta la direzione di una sfumatura lineare, con i pulsanti **+45°** e **−45°**.

Sotto l'anteprima c'è una guida con un indicatore per ogni punto. Fai clic sulla guida per aggiungere un punto, fai clic su un indicatore per aprirne il selettore, trascina un indicatore per spostarlo e fai clic destro su un indicatore per rimuoverlo. I tre campioni sotto la guida sono preimpostazioni: Sfumatura calda, Sfumatura fredda e Sfumatura di grigi. Le modifiche a una sfumatura si applicano alla selezione dal vivo.

La direzione di una sfumatura conta: orientala lungo la pendenza di un tetto o lungo un oggetto curvo, non attraverso facce che non hanno relazione tra loro. Dopo aver dipinto, controlla il modello da entrambi i lati.

## Materiale della superficie
**Materiale della superficie** imposta **Rugosità** (0 è lucido, 100 è opaco), **Metallicità** (0 per legno, pietra o plastica, 100 per il metallo) ed **Emissione** (da 0 a 8; la superficie brilla del proprio colore, sfumature comprese, ma non illumina gli oggetti vicini). Le modifiche si applicano dal vivo alle facce o ai brush selezionati. **Usa superficie predefinita** rimuove la personalizzazione. Tutte le modifiche si possono annullare.

Gli effetti della superficie sono visibili nella vista 3D illuminata e nei rendering della camera. La visualizzazione piatta e quella a reticolo tralasciano di proposito i riflessi speculari. Il bloom può aggiungere un alone attorno alle superfici emissive. Godot e glTF/GLB ricevono i valori del materiale, e l'importatore di Unity converte la rugosità in levigatezza. Vedi `docs/surface-materials.md` per i limiti di ciascuna destinazione.

# Asset, livelli e gruppi
## La libreria di asset
Apri la scheda **Asset**. Aggiungi una cartella di modelli `.map` con **Aggiungi cartella**; la libreria integrata è sempre presente. Digita in **Filtra gli asset** per cercare. Fai clic su un modello per selezionarlo, poi fai doppio clic, premi `Enter` o fai clic su **Inserisci** per aggiungerlo dove si trovava per ultimo il puntatore. Puoi anche trascinare un modello in una vista. I brush di un asset vengono inseriti come un unico gruppo, così il modello si sposta tutto insieme.

**Salva la selezione come asset** scrive i brush selezionati nella libreria come file `.map`. Gli asset della libreria sono normali brush modificabili, non mesh importate.

## Livelli
L'elenco **Livelli** della scheda Mappa organizza le aree estese. Digita un nome e fai clic su **Aggiungi** per creare un livello, oppure premi `Enter` per rinominare quello corrente. Ogni livello può essere nascosto, bloccato o escluso dalle esportazioni. **Sposta qui** sposta la selezione nel livello corrente. Il menu contestuale delle viste offre anche Sposta nel livello, Rendi livello attivo, Nascondi livelli e Isola livelli.

## Gruppi e gruppi collegati
`Ctrl+G` raggruppa la selezione e `Ctrl+Shift+G` scioglie il gruppo. La sezione **Gruppi** della scheda Mappa elenca i gruppi e offre **Apri**, **Rinomina**, **Unisci**, **Sciogli il gruppo**, **Togli dal gruppo** ed **Estrai**. Un gruppo collegato è un insieme di copie che condividono le modifiche: `Ctrl+Shift+D` crea un duplicato collegato, Geometria → Sincronizza i gruppi collegati aggiorna le copie e **Scollega** rende una copia indipendente. Nella scheda Entità, una proprietà può restare propria di una singola copia quando i gruppi collegati vengono sincronizzati.

## Una scala condivisa
La scala di esportazione predefinita è di 32 unità della mappa per metro. Il segnaposto integrato del punto di partenza del giocatore è alto 64 unità, un riferimento di 2 metri. Confronta con esso porte, veicoli, mobili e piante. Cambia la scala di esportazione solo intenzionalmente e mantieni coerenti gli asset correlati.

# Entità e gameplay
Le definizioni delle entità più comuni sono integrate e sempre disponibili: luci, punti di partenza del giocatore, porte, pulsanti, piattaforme, trigger, oggetti statici, suoni e waypoint. Caricando un FGD di progetto con **Definizioni delle entità** nella scheda Entità, le classi con lo stesso nome vengono sostituite e le altre classi integrate restano disponibili.

## Posizionare le entità
Nel browser **Entità** della scheda Entità, digita in **Filtra le classi** e scegli una classe. Per una classe puntuale, come una luce, fai clic su **Posiziona** o fai doppio clic sulla classe; `Insert` ne posiziona una al centro della selezione. Per una classe brush, come una porta o un trigger, seleziona i brush e fai clic su **Genera**. Le classi si possono anche trascinare in una vista.

## Proprietà
L'elenco delle proprietà modifica l'entità selezionata. Le entità della stessa classe vengono modificate insieme. **Aggiungi proprietà** aggiunge una chiave che la definizione potrebbe non conoscere; **Valori predefiniti** può riportare le proprietà esistenti ai valori predefiniti oppure aggiungere quelle mancanti. Le proprietà che la classe definisce ma che l'entità non ha sono mostrate attenuate; digitando un valore vengono aggiunte. Usa targetname e target per collegare le entità.

## Cosa può fare un'esportazione
Le direttive di Godot creano nodi nativi, corpi di collisione e aree trigger. Unity e Unreal supportano le luci comuni native, e Unreal crea anche attori PlayerStart. Gli altri dati vengono conservati per quanto lo consente l'importatore della destinazione.

Un'entità porta da sola non aggiunge alcun comportamento di gioco. Movimento, risposte ai trigger, controller del giocatore e script del progetto vanno comunque realizzati nel motore. Defold esporta le proprietà delle entità e gli oggetti di collisione; Flax posiziona modelli ed entità generiche. Controlla il risultato nel tuo motore prima di costruirci sopra il gameplay.

# Animazione
Heron Brush può mostrare l'anteprima di movimenti semplici e modificare animazioni a fotogrammi chiave delle entità brush, per poi esportarle verso diversi motori.

## Anteprima delle animazioni
Attiva **Anteprima animazioni** nel menu Vista, nel menu Animazione, nel menu Impostazioni griglia o nella sezione Vista della scheda Mappa. Riproduce la rotazione di func_rotate e il movimento avanti e indietro di func_shuttle quando start_on è attivo, usando le proprietà axis, speed, direction, travel e wait dell'entità, e riproduce le clip a fotogrammi chiave impostate su Riproduci all’avvio. Gli asset del ventilatore e del mulino a vento contengono già parti rotanti.

Disattivando l'anteprima si ripristina la posa originale. Un clic in una vista interrompe la riproduzione prima di selezionare o modificare; la navigazione della camera continua a funzionare. L'anteprima non modifica la mappa, la cronologia degli annullamenti né le esportazioni, e non esegue fisica né logica di gioco.

## L'editor dei fotogrammi chiave
Si apre con Animazione → Animazione con fotogrammi chiave oppure dal menu Impostazioni griglia. Si aggancia sotto le viste, oppure nella finestra 3D quando questa è separata. L'icona della finestra nella sua intestazione lo sposta in una finestra propria, mantenendo il fotogramma e la cronologia degli annullamenti. **Modifica animazione** porta le viste alla posa animata e **Modifica mappa** torna alla geometria di riposo. In modalità animazione, ogni vista mostra in basso **Modifica animazione** e il numero del fotogramma.

Seleziona un'entità brush, oppure un brush o un gruppo di brush, per animarlo. Un gruppo di brush semplice condivide un'unica animazione e un unico perno; i gruppi che contengono più entità richiedono animazioni separate.

## Clip
Il selettore **Animazione** passa da una clip con nome all'altra. Digita un nome in **Nome animazione** e fai clic su **Salva come** per copiare la clip corrente con quel nome; **Elimina** rimuove una clip. **Trasla** crea una clip `translate` senza rotazione. **Animazioni porta** crea le clip fluide `open` e `close` con una rotazione di 90 gradi attorno al perno corrente; prima sposta il perno sul cardine. **Riproduci all’avvio** fa partire la clip all'avvio del livello; la preimpostazione per le porte lo disattiva.

## Trasporto e riproduzione
I pulsanti di trasporto sono **Inizio**, **Chiave precedente**, **Riproduzione**, **Chiave successiva** e **Fine**. **Fotogramma** mostra e imposta il fotogramma corrente, e **Durata (s)** imposta la lunghezza della clip. La modalità è **Una volta**, **Ciclo** o **Avanti e indietro**.

## La timeline
La timeline ha un righello con la testina di riproduzione e una traccia per **Posizione** (X, Y e Z con chiavi comuni) e per ciascun angolo di rotazione, RX, RY e RZ. Fai clic o trascina nel righello per spostare la testina e vedere l'anteprima di quella posa in tutte le viste. Trascina con il tasto centrale per fare il pan della timeline. Trascina le chiavi per spostarle nel tempo; `Shift` aggiunge alla selezione; fai doppio clic su una traccia per inserire una chiave; `Ctrl`+rotellina zooma. **Curve** mostra le curve dei valori al posto delle tracce.

I pulsanti sopra di essa sono **Aggiungi chiave**, **Rimuovi**, **Copia**, **Incolla** e **Duplica**. **Aggancia ai fotogrammi** mantiene le chiavi su fotogrammi interi e **Adatta** mostra l'intera clip. **Modifica tempi chiavi** apre **Scala temporale**, **Spostamento (fotogrammi)**, **Scala dal cursore**, **Estendi durata** e **Modifica tempi selezione**.

L'ispettore delle chiavi accanto alla timeline modifica la chiave selezionata: **Fotogramma**, **Valore** (oppure le tre coordinate di una chiave di Posizione), l'interpolazione **Lineare**, **Costante** o **Bézier**, **Pendenza in entrata** e **Pendenza in uscita**, e **Smussa tangenti**.

## Tasti nell'editor
Quando la timeline ha la tastiera: `Space` avvia o mette in pausa la riproduzione, le frecce sinistra e destra avanzano di un fotogramma, `Ctrl+A` seleziona tutte le chiavi, `Ctrl+C`, `Ctrl+X` e `Ctrl+V` copiano, tagliano e incollano le chiavi, `Ctrl+D` le duplica, `Delete` le rimuove, `Ctrl+Z` annulla ed `Esc` annulla un trascinamento. Fuori dalla timeline, `Esc` esce dalla modalità animazione. L'annullamento lascia zoom, selezione e testina dove si trovavano.

## Mettere in posa nelle viste
La riga sotto la timeline contiene **Onion skinning** con un numero e **Spaziatura**, che mostra le pose vicine come fantasmi; le coordinate del **Perno**; **Chiavi automatiche**; e gli strumenti di posa **Trasla**, **Ruota** e **Perno**.

- **Aggiungi chiave** attiva Chiavi automatiche e mette in pausa la riproduzione. Con Chiavi automatiche attivo e **Trasla** scelto, trascina un asse colorato della maniglia dell'entità selezionata nella vista 3D per registrare una chiave della sua posizione al fotogramma corrente.
- In una vista 2D, trascina il brush selezionato o un membro del suo gruppo per registrare chiavi di traslazione lungo i due assi visibili, con aggancio alla griglia. Per questo non serve che Chiavi automatiche sia attivo.
- Con **Ruota**, trascina il brush in una vista 2D attorno al suo indicatore arancione del perno. Dall'alto registra RZ, Frontale RY e Laterale RX, con aggancio agli angoli della griglia.
- Con **Perno**, trascina il centro di rotazione giallo in una qualsiasi vista 2D. Si aggancia alla griglia e mantiene la sua profondità. Le chiavi e la geometria di riposo restano invariate.

Il percorso ciano nelle viste 2D e 3D mostra lo spostamento del perno. Fai clic su un punto del percorso per selezionarne il tempo e le chiavi, oppure trascinalo per cambiarne la posizione; con Chiavi automatiche attivo puoi trascinare i punti del percorso anche nella vista 3D. Le etichette mostrano i numeri dei fotogrammi e i segmenti tratteggiati indicano i salti. `Esc` annulla uno qualsiasi di questi trascinamenti, e un solo annullamento inverte un intero trascinamento.

Le nuove animazioni partono dalla posa di riposo con chiavi al tempo zero. Crea il movimento cambiando la posa in un altro fotogramma. Per un percorso netto a forma di L, registra chiavi all'inizio, all'angolo e alla fine, e usa Lineare con Avanti e indietro.

## Collegare pulsanti alle animazioni
Seleziona un func_button, un trigger_multiple o un trigger_once. La scheda Entità mostra **Attivatore animazione** sopra le proprietà. Scegli una destinazione dall'elenco, oppure fai clic su **Scegli destinazione nella vista** e poi su un oggetto animato in una qualsiasi vista; `Esc` annulla la scelta. Destinazioni o clip mancanti vengono segnalate sotto i controlli. I collegamenti tra entità mostrano frecce verso le loro destinazioni quando la visualizzazione dei collegamenti è attiva.

**Azione** stabilisce cosa fa l'attivazione: **Riproduci** avvia una clip con nome o l'animazione selezionata; **Alterna** alterna `open` e `close` quando esistono, altrimenti riavvia l'animazione selezionata; **Apri** e **Chiudi** avviano quelle clip; **Pausa** blocca la posa, **Riprendi** continua e **Ferma** ripristina la posa originale. **Una sola volta** sostituisce l'impostazione predefinita della classe. **Intervallo di ripetizione (s)** imposta il tempo minimo tra due attivazioni; non ritarda la prima.

**Prova attivatore** esegue l'azione nell'anteprima senza modificare la mappa né aggiungere passi di annullamento. **Reimposta anteprima** ripristina le pose originali e azzera gli stati di attivazione unica e di attesa. Non simula le collisioni del giocatore né gli script di gioco.

## L'animazione nelle esportazioni
Godot esporta risorse native AnimationPlayer e AnimationLibrary, comprese le curve e le collisioni in movimento. Un CharacterBody3D che entra nell'area di un pulsante attiva la sua destinazione. Le modifiche fatte a queste clip in Godot non tornano nella mappa e vengono sostituite dall'esportazione successiva.

L'esportazione raylib aggiunge `<map>_animation.h` con un player in C e `<map>_preview.c` con un'anteprima a camera libera; segui `<map>_raylib.md` per compilarla. Nell'anteprima `SPACE` mette in pausa, `R` riavvia le clip predefinite e `N` scorre le clip con nome. Defold integra la riproduzione nello script dell'entità con i messaggi documentati `heron_*_animation`. Build Level di Unity aggiunge `HeronBrushAnimation` e un corpo cinematico. L'esportazione Bevy include `heron_animation.rs` e le istruzioni. Godot, Unity, Defold, Bevy e raylib conservano i collegamenti dei trigger, l'indicatore di attivazione unica e l'intervallo di ripetizione. Il tuo gioco deve comunque registrare il giocatore e collegare i propri pulsanti all'API di ciascun runtime.

## L'esempio Motion lab
Apri `motion_lab` da Esempi nella schermata di benvenuto. Contiene una piattaforma con percorso a L, un ascensore, una piattaforma rotante e una porta a cardine con pulsanti. Apri in Godot il file `godot/project.godot` che si trova accanto e premi `F5`. WASD e il mouse muovono il giocatore, Spazio salta, da 1 a 4 si visitano le postazioni e R riavvia la postazione corrente. Sali su una piattaforma colorata per attivarne la destinazione. Il README incluso spiega come riesportare.

# Esportare verso un motore
File → Esporta apre la finestra Esporta. I motori sono elencati a sinistra con i loro loghi: Godot, Defold, three.js, Bevy, raylib, Unity, Unreal Engine, Flax Engine, Cocos Creator, Wavefront .obj, glTF .gltf, GLB .glb e per ultimo `.map`. La pagina a destra indica cosa riceverà la destinazione. **Scrivi** la scrive, **Verifica** costruisce il livello senza scriverlo ed elenca i problemi, e **Chiudi** chiude la finestra.

L'esportazione è separata dal salvataggio: il file `.map` resta la tua sorgente modificabile. Ogni controllo salva le impostazioni del proprio motore non appena cambia, così ogni motore conserva le sue scelte e la finestra si riapre come l'hai lasciata.

## Opzioni comuni
- **Scala**: **Unità per metro**, 32 per impostazione predefinita.
- **Cosa scrivere**: **Geometria di collisione**.
- **Ottimizzazioni**: **Raggruppa mesh statiche in celle di 1024 unità**, **Scarta le facce sepolte dentro la geometria**, **Unisci le facce che condividono un piano** e **Unisci le forme di collisione**.
- **Percorsi del progetto**: dove finiscono il livello e le sue risorse all'interno del progetto.
- **Materiali degli strumenti**: i nomi dei materiali clip, skip e origin.
- **Altro**: opzioni specifiche del motore come **Assegna nomi ai nodi in base a**, **Ricrea livelli e gruppi come nodi genitore**, **Inverti l'ordine dei vertici dei triangoli**, il ribaltamento delle coordinate texture e un secondo set UV per la luce precalcolata con **Texel della mappa di luce**.
- **Mantieni aggiornato con la mappa**: **Scrivi di nuovo a ogni salvataggio della mappa**. Diventa disponibile dopo la prima esportazione verso quel motore, perché è quell'esportazione a stabilire dove scrivere.

Le celle statiche assegnano i brush del mondo in base al loro centro e li combinano per materiale all'interno di ogni cella di 1024 unità, così i motori possono escludere dal rendering parti più piccole della mappa. La collisione viene mantenuta e le entità di gameplay restano separate. Può comportare più draw call rispetto a un'unica mesh del mondo, per questo è disattivata per impostazione predefinita. Le esportazioni autonome OBJ, glTF e GLB non usano le celle.

## Note sulle destinazioni
- Godot: i percorsi `res://` di texture e materiali, il livello fisico dei trigger e l'UV2 per le lightmap. Scegli una cartella di esportazione nel progetto Godot.
- Defold: la cartella delle risorse degli asset, il materiale del modello e il sampler delle texture, che deve esistere nel materiale scelto.
- Unity: mesh OBJ e un importatore per l'editor; esporta nella cartella Assets del progetto.
- Unreal Engine: uno script di importazione generato e un percorso dei contenuti, di solito /Game/<level>.
- Flax Engine: un percorso dei contenuti del modello relativo alla cartella Content del progetto.
- three.js, Bevy, raylib e Cocos Creator: scene o codice per questi runtime, con le istruzioni accanto all'esportazione.
- glTF / GLB: geometria portabile per gli strumenti compatibili; GLB è la forma binaria. OBJ: geometria e materiali per gli strumenti di modellazione.
- `.map`: **Scrivi come** sceglie tra Valve 220, Standard, Quake 2 o Quake 2 (Valve). La finestra indica cosa perderebbe il dialetto scelto.

Dopo la scrittura, leggi gli avvisi, ad esempio sulle texture mancanti. **Risultati della verifica** li elenca nella finestra.

# Collegamenti live: Godot, Unity, Defold e Flax
Un collegamento live invia le modifiche non salvate da Heron Brush all'editor di un motore mentre lavori. Funziona in una sola direzione: le modifiche fatte nel motore non tornano indietro. Per ogni finestra della mappa può essere attivo un solo collegamento live. Interrompilo prima di cambiare motore e riavvialo per applicare le opzioni di esportazione modificate. I controlli si trovano nella finestra Esporta, nella pagina del motore, nella scheda **Collegamento live con**.

L'indicatore nella barra del titolo mostra lo stato del collegamento; fai clic su di esso per interromperlo. Chiudendo o sostituendo la mappa la pubblicazione si interrompe. Dopo un errore di importazione, risolvi il problema e avvia di nuovo il collegamento.

## Godot
1. Esporta una volta in un progetto Godot.
2. Fai clic su **Installa estensione Godot**.
3. Attiva Heron Brush Live in Godot, in Impostazioni progetto → Plugin.
4. Fai clic su **Avvia collegamento live** e collega la mappa a una scena 3D aperta dal pannello di Godot.

L'estensione sostituisce solo il ramo che ha generato e mantiene la posizione del collegamento e gli altri nodi. Salva tu la scena di Godot, e tieni i tuoi script e nodi fuori dal ramo generato.

## Unity
1. Esporta una volta nella cartella Assets del progetto Unity.
2. Fai clic su **Installa estensione Unity** e lascia che Unity la compili.
3. In Unity, apri Window → Heron Brush → Live Link.
4. Avvia il collegamento e collega la mappa alla scena Unity attiva.

Tieni aperta la finestra live di Unity. Gli aggiornamenti si sospendono in Play Mode e in Prefab Mode. Viene sostituito solo il figlio Generated dell'oggetto mappa. Non modificare i file generati.

## Defold
1. Esporta una volta nel tuo progetto Defold e tieni il progetto aperto in Defold.
2. Fai clic su **Avvia collegamento live**. Non serve alcuna estensione.
3. Apri heron_brush_live/preview.collection in Defold, oppure richiamala dalla tua scena.

Il collegamento scrive in una cartella generata separata e non tocca l'esportazione finale né game.project. **Ricarica il gioco in esecuzione** aggiorna anche un gioco avviato da Defold; aggiungere oggetti o cambiare la collisione può richiedere il riavvio del livello. In caso di errori di connessione, i tentativi vengono ripetuti automaticamente.

## Flax
Esporta nella cartella Content del progetto e fai clic su **Installa componente Flax**. Lascia che Flax lo compili, avvia il collegamento, poi scegli Tools → Heron Brush: attach live map nella scena Flax aperta. La radice generata mantiene la sua posizione e gli altri oggetti non vengono toccati. Salva tu la scena. Gli aggiornamenti si sospendono in Play Mode. Tools → Heron Brush: stop live map sospende la ricezione.

# Sessioni collaborative dal vivo
Collaborazione → Sessione dal vivo apre una finestra per ospitare un altro utente di Heron Brush o unirti a lui. In questo modo più persone modificano la mappa insieme; è una funzione distinta dal collegamento live con un motore.

## Ospitare e partecipare
Inserisci **Il tuo nome**. Per ospitare, scegli **Ospita una sessione**, fai clic su **Avvia sessione** e invia ai tuoi ospiti i dati ottenuti con **Copia i dati di connessione**. Per partecipare, scegli **Partecipa a una sessione**, incolla i dati e fai clic su **Partecipa**. Partecipando, la mappa aperta viene sostituita da quella dell'host, quindi salva prima il tuo lavoro.

La build standard funziona sulla rete locale, e **Porta di rete locale** imposta la porta. Le sessioni via Internet (**Connetti tramite Internet**) richiedono una build con la funzionalità di rete opzionale. I partecipanti vedono a vicenda gli indicatori delle camere e i nomi nella vista 3D.

Chiudendo la finestra la sessione continua; l'indicatore nella barra del titolo ne mostra lo stato. **Termina sessione** o **Lascia la sessione** la chiude. Quando due persone modificano lo stesso oggetto, è l'ordine dell'host a decidere la versione finale, e anche l'annullamento è una modifica condivisa. Quando possibile, lavorate su aree separate e salvate dei punti di controllo.

# Visualizzazione 3D e secondo monitor
## Modalità di visualizzazione
La sezione **Vista** della scheda Mappa stabilisce cosa disegnano le viste: Brush, Entità puntuali, Modelli delle entità, Nomi di classe delle entità, Contorni dei gruppi, Patch, Materiali, Mostra facce omesse, Anteprima animazioni, Reticolo, Spigoli e Contorni delle entità brush. Reticolo disegna gli spigoli dei triangoli, comprese le diagonali e gli spigoli dietro le superfici, senza materiali. È una vista di debug temporanea e non influisce su esportazioni e rendering.

Illuminazione, ombre, occlusione ambientale, riflessi, bloom e antialiasing della vista 3D si trovano in Preferenze → Renderer. Sono impostazioni di anteprima; le immagini finali usano le impostazioni proprie della camera di rendering.

## Una finestra 3D separata
Vista → Vista 3D in una finestra separata sposta la vista 3D in una finestra propria, che puoi mettere su un altro monitor. Condivide camera, documento e strumenti, e le viste piane occupano lo spazio nella finestra principale. Il suo menu offre Materiali, Vista solida e Reticolo; un sottomenu Vista con Mostra facce omesse, Anteprima animazioni, Spigoli, Contorni dei gruppi, Entità puntuali, Modelli delle entità e Contorni delle entità brush; Metti a fuoco la selezione; Frontale, Laterale e Dall'alto, che ruotano la camera attorno alla selezione (o a un punto davanti alla camera); Posiziona la camera dalla vista 3D; Schermo intero (`F11`); Renderizza immagine; e Torna alla finestra principale. Chiudendo la finestra la vista torna agganciata.

## Cielo procedurale
Il cielo procedurale è attivo per impostazione predefinita nella vista 3D e si può disattivare in Preferenze → Renderer → Luce. Le camere di rendering hanno un proprio interruttore **Cielo procedurale**, salvato con la mappa. I colori di cielo, orizzonte e terreno provengono dalle impostazioni dell'ambiente, e il sole segue l'illuminazione della scena. Il cielo non ha geometria e non viene esportato.

# Camere di rendering e immagini PNG
File → Renderizza immagine apre la finestra di rendering: la camera e le sue impostazioni a sinistra e a destra un'anteprima dal vivo che si ridisegna man mano che le modifichi. Le camere vengono salvate nel file `.map` in un livello Render cameras riservato all'editor, escluso dalle esportazioni verso i motori.

## Posizionare la camera
Inquadra la scena nella vista 3D e premi `Ctrl+Alt+0` (o `Ctrl+Alt+Num0`), oppure fai clic su **Posiziona la camera dalla vista 3D**. Se non è selezionata alcuna camera, viene aggiornata la prima salvata; se non ne esiste nessuna, ne viene creata una. L'operazione si può annullare e mantiene le impostazioni di output e illuminazione. **Seleziona la camera nella mappa**, **Carica la camera dalla mappa** e **Sposta la vista 3D sulla camera** funzionano nella direzione opposta. **Cornice della camera nella vista 3D** mostra i limiti dell'immagine in uscita, e **Controlla la camera dalla vista 3D** permette alla normale navigazione di spostare la camera salvata.

## Impostazioni
- **Posizione della camera**: X, Y, Z, imbardata e beccheggio.
- **Output e obiettivo**: **Preimpostazioni** (Full HD, Quadrato, Verticale), larghezza, altezza, campioni, **Campo visivo verticale** e **Piano di ritaglio lontano**.
- **Illuminazione**: Luce diurna, Tramonto o Nuvoloso, oppure imposta esposizione, sole, colori di cielo, orizzonte e terreno, **Cielo procedurale**, **Ombre**, **Occlusione ambientale** e **Bloom**.
- **Profondità di campo**: **Attiva profondità di campo**, distanza di messa a fuoco, intervallo nitido e raggio di sfocatura. **Metti a fuoco la selezione** mette a fuoco il centro di un oggetto selezionato davanti alla camera.

**Salva camera e impostazioni** memorizza il modulo nella mappa; salva la mappa per conservarlo su disco.

## Rendering
**Renderizza PNG** chiede dove salvare e avvia un rendering progressivo, che si completa campione dopo campione nell'anteprima. Più campioni danno un aspetto migliore ma richiedono più tempo. **Annulla rendering** si interrompe senza scrivere alcun file. Il renderer usa la pipeline di rasterizzazione dell'editor, non il path tracing. Il PNG ha uno sfondo opaco e le animazioni non vengono renderizzate. Le immagini molto grandi possono bloccare brevemente l'editor.

# Preferenze
File → Preferenze (`Ctrl+,`) apre le impostazioni. Le modifiche si applicano subito e vengono salvate. La barra laterale elenca Panoramica, Giochi, Vista, Colori, Mouse, Tastiera e Renderer. Il campo di ricerca in alto trova un'impostazione per nome o descrizione in qualsiasi pagina. Ogni impostazione modificata ha un proprio pulsante per ripristinarne il valore predefinito, e **Ripristina i valori predefiniti** reimposta la pagina corrente.

## Le pagine
- **Panoramica**: le impostazioni comuni riunite: Aspetto (Opacità dei pannelli (%), Schema di colori, Disposizione, Lingua), Modifica, Vista e Manuale.
- **Giochi**: il gioco, dove si trovano i suoi file e il tuo nome per la collaborazione. Un gioco stabilisce quale formato di mappa scrivere, quali definizioni delle entità caricare e dove si trovano i suoi materiali. Le configurazioni dei giochi vengono lette dalla cartella `games` nella cartella di configurazione (`~/.config/heron-brush/games` su Linux e macOS, `%LOCALAPPDATA%\heron-brush\games` su Windows) e dalla cartella `games` accanto all’applicazione, dove i pacchetti mettono la configurazione inclusa.
- **Vista**: Disposizione, Schema di colori, Lingua, assi del mondo, opacità della griglia, dimensione delle celle per il controllo dei leak e i blocchi della mappatura.
- **Colori**: i colori delle viste, della geometria e delle maniglie.
- **Mouse**: velocità e inversioni per guardarsi intorno, il pan e lo spostamento, velocità di volo, campo visivo, Collega le viste 2D e dimensioni delle maniglie.
- **Tastiera**: la mappatura dei tasti, Heron Brush o Blender.
- **Renderer**: ombreggiatura realistica, contorno della selezione, luce, ombre, occlusione ambientale, riflessi, bloom e superfici.

## Lingua
**Lingua** offre Segui il sistema e tutte le lingue tradotte. L'interfaccia cambia subito. Il manuale si apre nella lingua dell'interfaccia quando ne esiste una traduzione, altrimenti in inglese.

# Plugin e MCP
## Plugin
I plugin estendono l'editor con script Rhai. File → Plugin apre la finestra Plugin. **Installa un plugin** installa un file `.rhai` e **Installa una cartella di plugin** installa una cartella che contiene `main.rhai`; i metadati facoltativi in `plugin.toml` descrivono il plugin e le sue impostazioni. I plugin sono raggruppati sotto **Plugin attivi** e **Plugin inattivi**, e **Abilita plugin** ne attiva o disattiva uno; la scelta resta valida anche dopo un riavvio. **Ricarica plugin** rilegge gli script dal disco e reimposta i valori dei loro pannelli.

I plugin attivi compaiono anche nella sezione **Plugin** della scheda Mappa con le loro impostazioni, i pulsanti delle azioni (**Esegui**) e l'output. Un'azione agisce sulla selezione, oppure su tutto quando non è selezionato nulla, ed è un singolo passo di annullamento. Il plugin incluso Selection inspector mostra i conteggi e le azioni per spostare in su e in giù.

## MCP
Collaborazione → Rendi disponibile a un agente (MCP) permette a un agente IA o a un altro strumento di controllare l'editor aperto tramite il Model Context Protocol. La finestra mostra l'indirizzo locale con **Copia**, e **Arresta server** lo chiude; mentre è in esecuzione, la barra del titolo mostra **MCP attivo**. L'opzione da riga di comando `--mcp` esegue la stessa interfaccia senza finestra. I client possono scoprire strumenti per geometria, asset, proprietà, animazione, esportazione e sessioni. MCP e i plugin dell'editor sono interfacce diverse.

# Riga di comando
Il programma è `heron-brush`. Senza opzioni apre la schermata di benvenuto.

## Aprire file
`heron-brush map.map [entities.fgd] [materials/]` apre una mappa, eventualmente con le definizioni delle entità e una cartella di materiali. Un argomento vuoto ne salta uno, per esempio `heron-brush map.map "" materials/`. Se un editor è già in esecuzione, la mappa si apre in quello; `--new-instance` apre invece una finestra separata.

## Comandi
- `--help` o `-h`: stampa le istruzioni d'uso. `--version` o `-V`: stampa la versione.
- `--mcp`: fornisce MCP tramite standard input e standard output, senza finestra.
- `--merge-map <base> <ours> <theirs> [<output>]`: un merge a tre vie di file `.map` per git. Per impostazione predefinita l'output è ours. I conflitti vengono stampati e producono un codice di uscita diverso da zero.
- `--obj-to-map <in.obj> <out.map> [size] [thickness] [snap] [material]`: trasforma una mesh in brush. Size è la lunghezza del lato più lungo in unità della mappa (predefinita 256), snap è una griglia per i vertici (0 per nessuna), e le parole `solid` o `layered` creano scatole a griglia o lastre convesse invece di cunei sottili.
- `--render-preview <in.map> <out.png> [size]`: scrive una miniatura PNG di una mappa, di 512 pixel per impostazione predefinita.
- `--run-script <script.rhai> <in.map> [out.map] [key=value ...]`: esegue uno script di plugin su una mappa con tutto selezionato e ne scrive il risultato (per impostazione predefinita nel file di input). `key=value` risponde alle impostazioni del plugin.

# Risoluzione dei problemi
## Quando qualcosa non va
- Materiale mancante: controlla la cartella o il pacchetto dei materiali e i percorsi del progetto nell'esportazione.
- Dimensioni del modello errate: confronta con il segnaposto del giocatore di 64 unità e con la scala di esportazione.
- Direzione della sfumatura errata: controlla l'angolo della sfumatura e l'orientamento della faccia.
- Scena del motore vuota: leggi gli avvisi dell'esportazione e segui le istruzioni di importazione della destinazione.
- Collegamento live in attesa: controlla che l'estensione sia installata, che la scena sia collegata e che gli aggiornamenti non siano sospesi.
- Solo linee, nessuna faccia: disattiva Reticolo nella sezione Vista della scheda Mappa, oppure scegli Materiali o Vista solida nel menu della finestra 3D.
- Oggetti mancanti nelle viste: `Ctrl+Shift+I` mostra gli oggetti nascosti; controlla i livelli e i filtri della sezione Vista.
- Un tasto non fa nulla: fai prima clic nella vista perché abbia la tastiera, e controlla la mappatura dei tasti in Preferenze → Tastiera.
- Una modifica non è riuscita: annulla, semplifica la geometria e usa **Controlla la mappa** e **Controlla i leak** nella sezione **Problemi** della scheda Mappa.
- Una mappa non si apre dal file manager: potrebbe essere stata passata a un editor già in esecuzione; cerca quella finestra.

## Impostazioni e backup
Preferenze, mappe recenti, stelle, impostazioni di esportazione e plugin si trovano nella cartella di configurazione: `~/.config/heron-brush` su Linux e macOS, e `%LOCALAPPDATA%\heron-brush` su Windows. I backup automatici sono nella cartella `autosave` accanto a ciascuna mappa.

## Segnalare un bug
Indica la versione dell'editor (Aiuto → Informazioni), il tuo sistema operativo, una piccola mappa che mostri il problema e i passaggi esatti. Il pannello di output (`Ctrl+4`) spesso indica cosa è andato storto. Salva una copia prima di fare esperimenti.

## Usare questo manuale
Scegli un capitolo nell'indice a sinistra. `Ctrl+F` mostra l'indice e passa al campo di ricerca; `Esc` cancella la ricerca, oppure chiude il manuale quando la ricerca è vuota. **Indice** nasconde o mostra l'elenco dei capitoli e **Dimensione testo** cambia la dimensione del carattere; entrambe le scelte vengono ricordate. L'elenco delle lingue in alto passa da una traduzione disponibile all'altra.
