# Başlarken
Heron Brush, fırça tabanlı bir seviye düzenleyicisidir. Seviyeleri dışbükey fırçalardan kurar, yüzeylerini boyar ve kaplarsınız; varlıklar yerleştirir, kapıları ve platformları canlandırır, sonucu da bir oyun motoruna aktarırsınız. Ayrıca yerleşik hazır modeller, parametrik şekiller, motor düzenleyicileriyle canlı bağlantılar, ortak düzenleme oturumları ve durağan görüntü işleme sunar. Bu kılavuz uygulamayla birlikte gelir ve çevrimdışı çalışır.

macOS'ta bu kılavuzda `Ctrl` geçen her yerde `Cmd` de çalışır; Option tuşu ise Alt'a karşılık gelir.

## Karşılama ekranı
Heron Brush bir harita olmadan başladığında karşılama ekranı açılır. Solda **<harita adı> ile devam et** (en son açtığınız harita), **Yeni harita** ve var olan bir `.map` dosyasını açmak için **Göz at**, ayrıca **Oyun**, **Kılavuz** ve **Tercihler** bulunur. Sağda küçük resimleriyle son haritaların listesi, onun altında da **Örnekler** yer alır. Yıldız (**Üstte tut**) bir haritayı listenin başına sabitler; çöp kutusu (**Çöp kutusuna taşı**) dosyayı sistemin çöp kutusuna taşır, böylece dosya geri yüklenebilir. Ok tuşlarıyla ekranda gezinirsiniz, Enter odaktaki öğeyi açar. Dosya → Hoş geldiniz bu ekranı istediğiniz zaman geri getirir.

## İlk haritanız
1. Karşılama ekranında **Yeni harita** düğmesini ya da Dosya → Yeni harita (`Ctrl+N`) komutunu seçin.
2. Araç şeridinde Fırça aracını (`B`) seçin ve seçili şekli çizmek için bir 2B görünümde sürükleyin. Derinliğini başka bir 2B görünümde kontrol edin.
3. Fırçayı seçin ve sürükleyerek taşıyın. Ona bir malzeme ya da renk vermek için **Yüzey** sekmesini kullanın.
4. Kaynak haritayı `Ctrl+S` ile kaydedin. Bir oyunda kullanmaya hazır olduğunuzda Dosya → Dışa aktar'ı açın ve motorunuz için bir sahne yazın.

## Dosyalar ve kaydetme
`.map` dosyası düzenlenebilir kaynağınızdır. Düzenli olarak kaydedin: dışa aktarma, canlı bağlantılar ve görüntü işleme haritayı hiçbir zaman sizin yerinize kaydetmez. Dosya → Farklı kaydet (`Ctrl+Shift+S`), uzantıyı yazmazsanız `.map` uzantısını ekler. Dosya → Kaydedilmiş haritaya dön kaydedilmemiş değişiklikleri atar ve dosyayı yeniden yükler. Kaydedilmemiş değişiklikler varken bir pencereyi kapatır ya da başka bir harita açarsanız Heron Brush ne yapılacağını sorar: **Kaydet**, **Vazgeç** ya da **İptal**.

Kaydedilmiş bir haritada kaydedilmemiş değişiklikler olduğu sürece, haritanın yanındaki `autosave` klasörüne on dakikada bir numaralı yedekler yazılır. En yeni 50 yedek saklanır.

Heron Brush zaten çalışırken bir harita açarsanız harita çalışan pencereye aktarılır ve bu pencere öne gelir. Ayrı bir pencere istediğinizde komut satırında `--new-instance` kullanın (bkz. Komut satırı).

# Arayüz
Ana pencerede menülerin bulunduğu bir başlık çubuğu, onun altında araç şeridi, harita görünümleri, sağda denetçi ve altta durum çubuğu bulunur.

## Başlık çubuğu ve menüler
Başlık çubuğunda logo, menüler, ortada harita adı ve pencere düğmeleri yer alır. macOS'ta alışılmış kırmızı, sarı ve yeşil düğmeler soldadır. Pencereyi taşımak için başlık çubuğunu sürükleyin, büyütmek için çift tıklayın. Arka planda bir şey çalışırken çubuğun sonuna yakın bir gösterge belirir: motor canlı bağlantısı, canlı oturum ya da **MCP etkin**. Canlı bağlantıyı durdurmak ya da ilgili pencereyi açmak için göstergeye tıklayın.

- **Dosya**: Hoş geldiniz, Yeni harita (`Ctrl+N`), Harita aç (`Ctrl+O`), Kaydet (`Ctrl+S`), Farklı kaydet (`Ctrl+Shift+S`), Kaydedilmiş haritaya dön, Dışa aktar, Görüntü işle, Eklentiler ve Tercihler (`Ctrl+,`).
- **Düzenle**: Geri al (`Ctrl+Z`), Yinele (`Ctrl+Shift+Z`), Kes, Kopyala, Yapıştır, Çoğalt (`Ctrl+D`), Sil, Tümünü seç (`Ctrl+A`), Son komutları tekrarla (`Ctrl+R`) ve Tekrarlanabilir komutları unut.
- **Görünüm**: Ayrı pencerede 3B görünüm, Git, Izgaraya tuttur (`Alt+0`), Izgarayı göster (`0`), Animasyonları önizle ve Atlanan yüzleri göster.
- **Geometri**: 3B metin ekle, Köşeyi birleştir, Yüzeyleri yamalara dönüştür, Seçimi varlığından çıkar ve Bağlantılı grupları eşitle.
- **Animasyon**: Anahtar kare animasyonu, Animasyonları önizle ve 3B görünümden kamera yerleştir (`Ctrl+Alt+0`).
- **Harita**: Nokta dosyası yükle, Nokta dosyasını yeniden yükle ve Nokta dosyasını kaldır; Portal dosyası yükle, Portal dosyasını yeniden yükle ve Portal dosyasını kaldır. Bunlar harita derleyicilerinin yazdığı sızıntı izlerini ve portalları gösterir.
- **İş birliği**: Canlı oturum ve Ajan için sunucu başlat (MCP).
- **Yardım**: Kılavuz ve Hakkında.

## Araç şeridi
Araç şeridinde tüm araçlar bulunur. İlk grup sürekli kullanılır: Seç, Fırça, Kes, Köşe, Kenar, Yüzey, Döndür ve Ölçek. İkinci grupta Eğ, Birleştir, Süpür, Kontrol noktası, Yuvarlat ve Boya yer alır. Bir düğmenin adını ve kısayolunu görmek için imleci üzerinde tutun. Etkin araca yeniden tıklamak Seç aracına döndürür.

Araçlardan sonra **Yatay çevir** ve **Dikey çevir**, ardından iki anahtar gelir: **Kilitle** bir fırça taşındığında malzeme eşlemesini korur, **UV** ise fırça döndüğünde korur (`U` ile açılıp kapanır). Sonra etkin aracın kendi ayarları gelir: Fırça aracı şekli ve seçeneklerini, Yuvarlat aracı ise yarıçapını, yüzey ve köşe ayarlarını **Uygula** düğmesiyle birlikte gösterir. Sağ uçta araç adı, ızgara boyutu (**Izgara 16** vb.; başka bir boyut seçmek için tıklayın) ve Izgaraya tuttur, Izgarayı göster, Animasyonları önizle, Atlanan yüzleri göster ve Anahtar kare animasyonu öğelerini içeren **Izgara ayarları** menüsü bulunur.

## Harita görünümleri
Dört görünüm vardır: 3B görünüm ve üç düz görünüm, yani Üst (XY), Ön (XZ) ve Yan (YZ). Tercihler → Görünüm → Yerleşim ile Tek bölme, İki bölme, Üç bölme ya da Dört bölme seçilir. Bölmeleri yeniden boyutlandırmak için ayırıcıları sürükleyin. Bir görünüme tıklamak klavyeyi ona verir. `Space`, tek seferde bir görünüm gösteren bir bölmeyi sıradaki görünüme geçirir; `Ctrl+Space` klavyenin bulunduğu görünümü tüm alana büyütür ve tekrar basıldığında yerleşimi geri getirir.

Herhangi bir görünümde sürüklemeden sağ tıklayarak seçim ve imlecin altındaki nesne için bir bağlam menüsü açın. Bu menü gruplama, katmanlar, bağlantılı gruplar, varlık oluşturma, fırçaları bir varlığa taşıma ya da varlıktan çıkarma, malzeme adını kopyalama ve malzemeyi malzeme tarayıcısında gösterme seçeneklerini sunar.

## Denetçi
Sağdaki denetçinin dört sekmesi vardır. Genişliğini değiştirmek için sol kenarını sürükleyin.

- **Harita** (`Ctrl+1`): Katmanlar, Gruplar, Eklentiler, Sorunlar, Görünüm bölümündeki filtreler ve Etiketler.
- **Varlık** (`Ctrl+2`): varlık tarayıcısı, varlık tanımları, seçili varlığın özellikleri ve düğmeler ile tetikleyiciler için Animasyon tetikleyicisi.
- **Yüzey** (`Ctrl+3`): UV, Malzemeler, Renk, Süpür, Yuvarlat, Yama ve İçe al ve pah kır. Her bölüm daraltılabilir.
- **Öğeler**: yeniden kullanılabilir fırça modellerinden oluşan kitaplık.

`Ctrl+5` denetçiyi, `Ctrl+Alt+T` ise araç şeridini gizler ya da gösterir; böylece görünümlere daha fazla yer kalır.

## Durum çubuğu ve çıktı bölmesi
Alttaki durum çubuğu düzenleyicinin son iletisini gösterir. Hatalar kırmızıyla yazılır. Tüm ileti geçmişini içeren çıktı bölmesini açmak için çubuğa tıklayın ya da `Ctrl+4` tuşuna basın. Çoğu komut ne yaptığını burada bildirir; örneğin kaç fırçanın kesildiğini.

## İletişim kutuları ve görünüm
Tercihler, Dışa aktar ve Görüntü işle gibi iletişim kutuları kendi pencerelerinde açılır. Tercihler → Görünüm → Renk şeması ile Masaüstünü izle, Açık ya da Koyu seçilir. Genel bakış sayfasındaki **Panel opaklığı (%)** panelleri görünümlerin üzerinde yarı saydam yapar. Pencere kendi düğmelerini sisteminizin tarzında çizer: macOS'ta yerel, Windows'ta Windows 11 tarzında, Linux'ta GNOME tarzında.

# Gezinme ve klavye
## 3B görünümde hareket
Etrafa bakmak için sağ fare tuşunu basılı tutup sürükleyin. Kaydırmak için orta tuşla sürükleyin. Tekerlek ileri ve geri hareket ettirir. `W`, `A`, `S` ve `D` ileri, sola, geri ve sağa uçurur; `Q` yükseltir, `X` alçaltır; daha büyük adımlar için `Shift` tuşunu basılı tutun. Tercihler → Fare'de **Alt + orta tuşla sürükleme kamerayı hareket ettirir** açıkken Alt ile orta tuş görüş yönü boyunca hareket ettirir. Hızlar, ters çevirmeler ve görüş alanı Tercihler → Fare altındadır.

## 2B görünümlerde hareket
Kaydırmak için orta tuşla sürükleyin, imlecin çevresinde yakınlaştırmak için tekerleği kullanın. Bir düz görünümü kaydırmak ve yakınlaştırmak diğerlerini de hizalı tutar; bunu Tercihler → Fare'deki **2B görünümleri bağla** denetler. `Ctrl+Alt+Z` yakınlaştırmayı sıfırlar, `Ctrl+Shift+U` düz görünümleri başlangıç noktasına döndürür.

`Ctrl+U` seçimi tüm görünümlerde çerçeveler. Görünüm → Git (`Ctrl+Alt+M`) kamerayı yazılan koordinatlara taşır, harita dosyasının belirli bir satırındaki öğeyi seçer ya da seçimi yazılan miktar kadar taşır.

## Izgara ve tutturma
`+` ve `-` ızgara boyutunu değiştirir; `1` ile `9` arasındaki rakam tuşları 1 ile 256 birim arasında bir boyutu doğrudan seçer. `0` ya da `#` ızgarayı gösterir veya gizler, `Alt+0` tutturmayı açar ya da kapatır. `S` seçili fırçaların köşelerini geçerli ızgaraya tutturur; `Ctrl+Shift+V` tam birimlere, `Ctrl+Alt+Shift+V` ise ızgaraya tutturur.

## Sık kullanılan kısayollar
- `F3` veya `Ctrl+Shift+P`: herhangi bir menü komutunu adıyla arar, `Enter` çalıştırır. `F10` menüleri açar, ok tuşları menülerde gezinir.
- `Ctrl+S`: haritayı kaydet. `Ctrl+Shift+S`: farklı kaydet.
- `Ctrl+Z`: geri al. `Ctrl+Shift+Z` ya da `Ctrl+Y`: yinele.
- `Ctrl+C`, `Ctrl+X`, `Ctrl+V`: kopyala, kes ve yapıştır. Yapıştırma nesneleri imlecin bulunduğu yere koyar; `Ctrl+Alt+V` özgün koordinatlarını korur. Yüzeyler seçiliyken kopyalama onların malzeme ve UV ayarlarını alır.
- `Ctrl+D`: çoğalt. `Ctrl+Shift+D`: bir grubun bağlantılı kopyasını oluştur.
- `Delete` ya da `Backspace`: seçimi sil.
- `Esc`: geçerli sürüklemeyi iptal eder, etkin araçta bir adım geri gider, Seç aracına döner ve en sonunda seçimi temizler. `Shift+Esc` doğrudan Seç aracına döner.
- `Ctrl+R`: son tekrarlanabilir komutları tekrarla. `Ctrl+Shift+R`: bu komutları unut.
- `Ctrl+Alt+0` ya da `Ctrl+Alt+Num0`: işleme kamerasını 3B görünümün konumuna yerleştir.
- `F5`: malzemeleri yeniden yükle. `F6`: varlık tanımlarını yeniden yükle.

## Seçim kısayolları
- `Ctrl+A`: tümünü seç; köşe, kenar ya da yüzey aracı etkinken tüm tutamaçları seç. `Ctrl+Shift+A`: seçimi kaldır. `Ctrl+Alt+A`: seçimi tersine çevir.
- `Ctrl+B`: aynı varlık ya da gruptaki diğer nesneleri seç.
- `Ctrl+T`: seçili fırçaların dokunduğu her şeyi seç. `Ctrl+E`: içlerinde kalan her şeyi seç. `Ctrl+Shift+E`: görünüm ekseni boyunca hizalarındaki her şeyi seç. Bu komutlar seçimde kullanılan fırçaları tüketir.
- `Ctrl+I`: seçimi yalıt. `Ctrl+Alt+I`: seçimi gizle. `Ctrl+Shift+I`: her şeyi yeniden göster.
- `Ctrl+G`: grupla. `Ctrl+Shift+G`: grubu çöz.

## Dönüştürme kısayolları
- Ok tuşları ile `Page Up` / `Page Down` seçimi, etkin görünümde gösterdikleri yönde bir ızgara adımı taşır. `Ctrl` ile taşırken çoğaltır, `Alt` ile döndürür.
- `Alt+X`, `Alt+Y`, `Alt+Z`: o eksen etrafında çeyrek tur döndür; ters yön için `Shift` ekleyin.
- `Ctrl+F`: görünüm boyunca çevir; `Ctrl+Alt+F`: yukarı-aşağı çevir. `Ctrl+Shift+X` ve `Ctrl+Shift+Y` X ya da Y boyunca aynalar.
- `Insert`: seçimin merkezine bir nokta varlığı yerleştir.
- `Tab`: bir fırça seçiliyken Seç → Köşe → Kenar → Yüzey → Seç sırasıyla dolaş.

## Blender tuş eşlemesi
Tercihler → Klavye → Tuş eşlemesi; Heron Brush (varsayılan, bu kılavuz boyunca kullanılan) ve Blender seçeneklerini sunar. Blender ile: `G` seçip taşır, `R` döndürür, `S` ölçekler, `E` süpürür, `1`, `2` ve `3` köşe, kenar ve yüzey araçlarını seçer, `K` keser, `B` seçer, `A` her şeyi seçer ve `Alt+A` seçimi bırakır, `X` siler, `Shift+D` çoğaltır, `H` gizler ve `Alt+H` her şeyi gösterir, `Ctrl+Shift+Z` ise yineler. Geri kalan her şey ikisinde de aynıdır.

# Fırçalar ve modelleme araçları
Fırçalar, düzlemlerle tanımlanan dışbükey katılardır. Tek bir fırçayı içbükey yapmaya çalışmak yerine karmaşık nesneleri birkaç basit fırçadan oluşturun. Basit bir şeklin yettiği yerde fırça sayısını düşük tutun.

## Fırça çizme
Fırça aracını (`B`) seçin ve bir görünümde sürükleyin. Araç şeridinde **Çizilecek şekil** görünür: Dikdörtgenler prizması, Merdiven, Kemer, Silindir, Koni, Sferoit (UV), Sferoit (ikozahedron) ve 3B metin ekle. `Alt+1` ile `Alt+7` arası tuşlar bir şekli doğrudan seçer. Şekle göre şeritte **İçini boşalt** ve **Kalınlık:**, merdivenler için **Basamak yüksekliği:** ve **Yön:**, yuvarlak şekiller için **Eksen:**, **Kenar sayısı:** ve daire kipi, küreler için **Halkalar:**, ikosahedron için de **Hassasiyet:** görünür. Seçenekleri sürüklemeden önce ayarlayın. Sonucu birden fazla görünümde kontrol edin.

## Seçme
Bir fırçayı seçmek için tıklayın, seçime eklemek için `Ctrl` ile tıklayın. Boş bir alandan başlayan sürükleme bir seçim kutusu çizer. 3B görünümde `Shift` ile tıklama tek bir yüzey seçer, `Ctrl+Shift` ile tıklama başka yüzeyler ekler. Seçili bir fırçayı sürüklemek onu taşır; seçili olmayan bir fırçayı sürüklemek yalnızca o fırçayı seçip taşır.

3B görünümde seçili bir fırçanın bir yüzeyini `Shift` ile sürükleyerek dışarı doğru uzatın (ekstrüzyon). Seçili yüzeyin malzemesini ve hizalamasını başka bir yüzeye kopyalamak için o yüzeye `Alt` ile tıklayın; `Alt+Ctrl` yalnızca malzemeyi, `Alt+Shift` ise dönüşü kopyalar.

## Şekillendirme araçları
- **Seç** (`Esc`): seçin ve taşıyın.
- **Köşe** (`V`), **Kenar** (`E`), **Yüzey** (`F`): köşeleri, kenarları ya da yüzeyleri sürükleyin. Bir kenarın ya da yüzeyin ortasından `Shift` ile sürüklemek oraya yeni bir köşe ekler. `Delete` seçilen köşeleri kaldırır.
- **Döndür** (`R`), **Ölçek** (`T`), **Eğ** (`G`): seçimi tutamaçlarla dönüştürün.
- **Kes** (`C`): bir kesme düzlemi tanımlamak için iki ya da üç noktaya tıklayın. `Enter` keser, `Ctrl+Enter` hangi tarafın kalacağını seçer, `Backspace` ya da `Esc` son noktayı kaldırır.
- **Birleştir** (`N`): bir hacmin çevresine noktalar yerleştirin ve bunlardan dışbükey bir fırça oluşturmak için `Enter` tuşuna basın. `Backspace` son noktayı kaldırır.
- **Süpür** (`Y`): aşağıdaki Süpürme bölümüne bakın.
- **Kontrol noktası** (`P`): bir yamanın kontrol ızgarasını yeniden şekillendirin.
- **Yuvarlat** (`O`): yuvarlatmak için bir kenara tıklayın ya da kenarları veya köşeleri kendi araçlarıyla seçip `Enter` tuşuna basın. Daha fazla yüzey daha pürüzsüz bir sonuç ama daha fazla geometri demektir.
- **Boya** (`K`): bkz. Malzemeler ve UV'ler.

## Fırçaları birleştirme
- `Ctrl+J`: seçili fırçaları dışbükey zarflarında birleştir.
- `Ctrl+K`: seçili fırçaları dokundukları fırçalardan çıkar.
- `Ctrl+Shift+K`: seçimin içini bir ızgara adımı kalınlığında duvarlar bırakarak boşalt.
- `Ctrl+L`: seçimin kesişimini al.

## Süpürme, yuvarlatma, yama ve pah
**Süpür**, seçili yüzeylerden bir yol boyunca geometri oluşturur. Yüzeyleri seçin (Süpür aracında `Alt` ile tıklayarak), ardından Yüzey sekmesinin Süpür bölümünde **Yol mesafesi**, **Döndürme** (eksen ve açı), **Ölçek**, **Adımlar** ve **Yol** (Tam sayı ya da Serbest hizalama; Yay, Düz ya da S kıvrımı) değerlerini ayarlayın ve **Süpür** düğmesine basın. Denetimlerin altındaki satır kaç fırça oluşturulacağını söyler.

**Yuvarlat** bölümü seçilen kenarları **Yarıçap**, **Yüzeyler** ve **Köşeler** (Gönyeli ya da Yuvarlatıldı) ayarlarıyla yuvarlatır; tek yüzey bir pah oluşturur.

**Yama** bölümü seçili yamanın **Sütunlar** ve **Satırlar** değerlerini, birer çift ekleyen ya da kaldıran düğmelerle birlikte gösterir. Geometri → Yüzeyleri yamalara dönüştür (`Ctrl+P`) seçili yüzeylerden yamalar oluşturur.

**İçe al ve pah kır** seçili yüzeylerde çalışır: **İçe al** yüzeyi kenarlarından içeri çeker, **Derinlik** ise onu dışarı (negatifse içeri) iter. **Yüzeylere pah kır** ikisini birden uygular; üçüncü düğme Derinlik değerinin işaretine göre bir panel ekler ya da bir girinti oyar. **Köşeyi birleştir**, iki fırçayı gönyeli ya da yuvarlatılmış bir köşede önizlemeyle birleştiren bir iletişim kutusu açar.

## 3B metin
Geometri → 3B metin ekle (Fırça aracının şekil listesindeki son öğe de budur) metinden fırçalar oluşturur. Köşeleri kırpılmış, orantılı genişlikli ve Çekçe aksanları olan yerleşik bir çokgen yazı tipi olan Heron Block'u kullanır; küçük harfler büyük harf biçimleriyle çizilir. **Metin** alanına yazın, **Yükseklik** (büyük harf yüksekliği; aksanlar bunun üstüne çıkar) ve **Derinlik** değerlerini ayarlayın, canlı önizlemeyi izleyin ve **Ekle** düğmesine basın.

Bir işlem geçerli bir geometri üretemezse boyutlarını değiştirin ya da işlemi geri alıp şekli sadeleştirin.

## Diziler
Geometri → Dizi seçimi kopyalar ve değerleri değiştirirken kopyaları haritada gösterir. Sayılara asıl nesne de dahildir ve bir dizi en fazla 1024 kopya oluşturur.

- **Sıra** her kopyayı bir öncekine göre **X adımı**, **Y adımı** ve **Z adımı** kadar kaydırır ve seçilen eksen çevresinde **Kopya başına dönüş (derece)** kadar döndürür.
- **Izgara**, **X adımı**, **Y adımı** ve **Z adımı** aralıklarla **Sayı X**, **Sayı Y** ve **Sayı Z** öğe yerleştirir.
- **Halka** öğeleri seçimden **Yarıçap (birim)** uzaklıktaki bir merkezin çevresine **Açı (derece)** boyunca dağıtır. **Merkeze döndür** açıkken her kopya halkayla birlikte döner.
- **Dağılım** kopyaları yarıçap içinde, seçilen eksene dik rastgele yerlere koyar. Aynı **Tohum** aynı yerleşimi verir; **Rastgele dönüş** her kopyayı döndürür.

Seçim tek bir grubun tamamıysa **Bağlı kopyalar** her kopyayı bağlı bir kopya yapar, böylece birinde yapılan değişiklik hepsine ulaşır. **Uygula** kopyaları tutar ve onları asıl nesneyle birlikte seçer; **İptal** onları kaldırır. Dizinin tamamı tek adımda geri alınır.

## Arazi şekillendirme
Arazi, şekillendirme fırçasının yüzeyini dışarı itip içeri çektiği çok sayıda küçük brush'tan oluşur. Bir veya daha fazla brush seçin, Geometri → Sütunlara böl komutunu seçin, ardından sütunların biçimini seçin: Üçgen, Kare veya Altıgen. Komut onları ızgara çizgileri boyunca bir ızgara adımı genişliğinde sütunlara böler ve sütunları Terrain adlı bir gruba koyar. Kesimler dünyanın ızgarasını izler, bu yüzden yan yana duran brush'lar aynı yerlerden kesilir ve köşelerini paylaşır. Her sütun brush'ın ince kenarı boyunca durur: zeminde veya tavanda dik, duvarda yatay. Mağara da aynı şekilde yapılır: bir kutuyu bir ızgara adımı duvar kalınlığıyla oyun (Hollow selection), duvarları seçin ve bölün.

Seçimi, şekillendirme şeridinin sonundaki düğmenin açtığı **Şekillendir** penceresinden de bölebilirsiniz; sütunlar, pencerenin Sütun biçimi satırında seçili biçimi alır. İki yer de son kullanılan biçimi hatırlar. **Üçgen** her ızgara hücresine iki sütun koyar ve üçgenin üstü nasıl şekillendirilirse şekillendirilsin düz kalır. **Kare** her hücreye bir sütun koyar, yani yarı sayıda brush; üstü bir köşegen boyunca bükülür. **Altıgen**, brush'ı iki ızgara adımı genişliğinde altıgenlerle kaplar; her biri merkezinin çevresinde altı üçgene bölünür, böylece her üst yüz düz kalır.

**Şekillendir** aracını (`J`) seçin ve 3D görünümde yüzeyin üzerinde sürükleyin. Fareyi oynatmadan düğmeyi basılı tutarsanız fırça aynı noktada saniyede on kez çalışmayı sürdürür, böylece siz tuttukça bir tepe yükselir. Turuncu halka fırçayı gösterir. Seçim yokken fırça altındaki her brush'ı değiştirir; seçim varken yalnızca seçilenleri. Pencerenin üstündeki şerit bir darbenin ne yapacağını seçer:

- **Yükselt** ve **Alçalt** yüzeyi dışarı veya içeri taşır; halkanın ortasında en çok, kenarında hiç. Yer değiştirmek için `Shift` tuşunu basılı tutun.
- **Yumuşat** tümsekleri düzeltir.
- **Düzleştir** yüzeyi darbenin başladığı yere düzler.

Aynı pencerede **Yön** hangi yöne gidileceğini seçer:

- **Yukarı** yüzeyi yerdeki bir daire içinde dümdüz yukarı ve aşağı taşır. Arazi için budur.
- **Kamera** onu işaretçinin çevresindeki bir küre içinde görünüme doğru ve görünümden uzağa taşır. Bir zemine, duvara ya da tavana bakın, fırça onu şekillendirir.
- **Normal** onu, darbe başladığında işaretçinin altındaki yüzden dışarı taşır.

İki sayı **Yarıçap (birim)** ve **Güç (%)** değerleridir. Yalnızca darbenin taşıdığı taraftaki köşeler hareket eder; arka taraf yerinde kalır ve bir sütun en az bir birim kalınlıkta kalır. Brush'lar paylaştıkları bir köşeyi birlikte taşır, böylece yüzey kapalı kalır. Bir darbe tek adımda geri alınır. 2D görünümlerde araç, Seçim aracı gibi seçer.

Şekillendirme bittiğinde araziyi seçin ve Geometri → **Sütunları birleştir** komutunu ya da **Şekillendir** penceresindeki düğmeyi seçin. Birlikte dışbükey bir brush oluşturan ve aynı görünen komşu sütunlar tek bir brush olur; böylece düz bölgeler birkaç büyük brush'a dönüşür. Yüzey kıpırdamaz ve birleştirme tek adımda geri alınır.

Voksel oyunlarındaki gibi bir arazi için **Kare** sütunlara bölün ve **Şekillendir** penceresinde **Darbe** satırından **Bloklar** seçeneğini seçin. Bir fırça darbesi o zaman her sütunun tüm üstünü tam ızgara adımlarıyla taşır ve yanlarını dik bırakır; zemin basamak basamak yükselir. Tüm kipler eskisi gibi çalışır; küçük darbeler bir sütun tam bir adım yükselene kadar toplanır. Başka biçimdeki sütunlara dokunulmaz.

**Serbest** sütun gerektirmez; her brush üzerinde çalışır. Yükselt, işaretçinin çevresindeki bir kürede zemin ekler, Alçalt ise çıkarır; böylece zemin çıkıntı yapabilir ve mağaralar oluşabilir. Yumuşat düzleştirir, Düzleştir ise darbenin başladığı seviyeye getirir. Her dokunuş, ulaştığı brush'ları ızgarada yuvarlatılmış olarak yeniden kurar ve kürenin dışındaki her şeyi, keskin kenarlar dahil, olduğu gibi bırakır. Sonuç sıradan brush'lardır; tüm diğer araçlar onlarla çalışır ve **Sütunları birleştir** sonradan sayılarını azaltır. Daha sık bir ızgara daha pürüzsüz bir yüzey ve daha çok brush verir.

# Malzemeler ve UV'ler
## Malzemeleri yükleme ve uygulama
Yüzey sekmesinin **Malzemeler** bölümünü açın. **Klasör yükle** bir görüntü klasörü, **.wad yükle** ise bir doku paketi ekler; ikisi de **Koleksiyonlar** altında görünür ve çarpı işaretiyle kaldırılabilir. Listeyi daraltmak için **Malzemeleri süz** alanına yazın. Bir malzemeyi geçerli malzeme yapmak için üzerine tıklayın; ardından **Seçime uygula** onu tüm seçili yüzeylere uygular. **Seç** menüsü, Bu malzemeyi taşıyan fırçaları seç ve Bu malzemeyi taşıyan yüzeyleri seç seçeneklerini sunar. **Değiştir** bir malzemeyi tüm haritada ya da yalnızca seçimde başka bir malzemeyle değiştirir.

**Boya** aracı (`K`) geçerli malzemeyi imlecin altındaki yüzeye uygular: tek tıklama bir yüzeyi, çift tıklama tüm fırçayı boyar; `Shift` ile tıklama ise damlalık gibi yüzeyin malzemesini alıp geçerli malzeme yapar.

## UV düzenleyicisi
Yüzey sekmesinin **UV** bölümü seçili yüzeyin malzemesini ve ana hattını gösterir. Tuval üzerindeki fare denetimleri:
- Tekerlek imlecin çevresinde yakınlaştırır. Kaydırmak için sağ ya da orta tuşla sürükleyin. Yüzeyi yeniden çerçevelemek için sağ tuşla çift tıklayın; başka bir yüzey seçildiğinde o yüzey otomatik olarak çerçevelenir.
- Sol tuşla sürüklemek, yüzeyin ana hattı yerinde kalırken malzemeyi taşır. Halka eşlemeyi döndürür, köşe tutamaçları ölçekler, merkez tutamacı ise bu değişikliklerin merkez noktasını belirler.
- Bir döşemenin kenarında `Alt` ile sol tuşla sürüklemek eğme yapar; `Shift` ile orta tuşla sürüklemek de aynı işi görür.
- Her sürükleme bir geri alma adımıdır; `Esc` süren bir sürüklemeyi iptal eder.

Tuvalin altında **Kaydırma**, **Ölçek**, **Açı** ve oyun kullanıyorsa **İçerik bayrakları**, **Yüzey bayrakları** ve **Yüzey değeri** bulunur. Düğmeler şunlardır: **Sıfırla**, **Dünya hizalamasına sıfırla**, **Sığdır**, **Yatay çevir**, **Dikey çevir** ve iki 90° döndürme düğmesi. **Hizala** malzemeyi yüzeyin kenarlarından biri boyunca döndürür (sonraki kenar için yeniden basın, önceki kenar için `Shift` kullanın). Dört yaslama düğmesi malzemeyi bir kenara dayar. **U yönünde sığdır** ve **V yönünde sığdır**, yüzeyi tam sayıda kopya kaplayacak şekilde ölçekler; sonraki sayı için yeniden basın, bir trim sheet'in alt bölümlerini korumak için `Ctrl` tuşunu basılı tutun.

## UV tuşları
Yüzeyler seçiliyken ok tuşları malzemeyi taşır, `[` ve `]` onu 15° döndürür, `,` ve `.` ölçeğini yarıya indirir ya da iki katına çıkarır, `Ctrl+F` ve `Ctrl+Alt+F` çevirir, `Shift+R` sıfırlar, `Alt+Shift+R` dünya eksenlerine göre sıfırlar ve `Ctrl+Shift+F` her yüzeye sığdırır.

Araç şeridindeki **Kilitle** fırçalar taşınırken, **UV** (`U`) ise fırçalar dönerken eşlemeyi korur.

## Araç malzemeleri
- skip: işaretlenen yüzeyi motor dışa aktarımlarından çıkarır.
- clip: görünür geometri olmadan çarpışma oluşturur.
- origin: bir fırça varlığının dönme merkezini işaretler.

Bu adlar Dışa aktar penceresindeki **Araç malzemeleri** kartında değiştirilebilir. Gizli kalacağını bildiğiniz yüzeyler için skip kullanın.

## Yüzeyleri dışa aktarmadan çıkarma
Bir yüzey seçin ve UV bölümünde **Yüzü dışa aktarmadan çıkar** seçeneğini açın. UV önizlemesinde turuncu tarama bu yüzeyi işaretler. Malzemesi ve boyası kayıtlı kalır, fırça da kapalı ve düzenlenebilir olmaya devam eder. Dışa aktarılan görsel ağlar bu yüzeyi dışarıda bırakır, çarpışma ise korunur. Bir küpün tek bir dörtgen yüzeyini dışarıda bırakmak iki görsel üçgen eksiltir. Bu seçenek yüzeyleri çift taraflı yapmaz ve ağları birleştirmez.

**Kapalı yüzleri atla**, statik fırçalarla tamamen örtülen yüzeyleri işaretler; birbirine değen özdeş yüzeyler ve tamamen kapatılmış fırçalar da buna dahildir. Seçili fırçalarda, hiçbir şey seçili değilse de görünür ve kilitsiz tüm dünya geometrisinde çalışır. Fırça varlıkları, saydam malzemeler ve zaten atlanmış yüzeyleri olan fırçalar es geçilir. Kısmen örtülen yüzeyler bölünmez. Tek bir geri alma her şeyi eski haline getirir. Görünüm → Atlanan yüzleri göster bu yüzeyleri düzenleyici görünümlerinde gizler ya da gösterir; seçme işlemi yine tüm fırçayı kullanır.

# Renk ve renk geçişleri
Yüzey sekmesinin **Renk** bölümü yüzeyleri malzemelerinin üzerine düz renklerle ya da renk geçişleriyle boyar. Bir seviyenin taslağını çıkarmak ve stilize çizimler için kullanışlıdır. Boyanmış yüzeyler görünümlerde gerçek renklerini korur; seçim ana hatlarıyla gösterilir.

## Palet ve karışım rengi
Seçili yüzeyleri ya da seçili fırçaların tamamını boyamak için on iki palet rengi örneğinden birine tıklayın. **Opaklık** bir sonraki boyanın ne kadar opak olacağını yüzde olarak belirler. **Karışım** satırında renk seçiciyi açan bir renk düğmesi bulunur; seçicide sürüklediğinizde seçim canlı olarak güncellenir, karışım rengini tek adımda uygulamak içinse **Boya** düğmesine basın. **Malzemeye dön** boyayı kaldırır ve yüzey yeniden malzemesini gösterir. Bölümün üstündeki satır neyin seçili olduğunu ve üzerinde ne bulunduğunu söyler.

## Renk seçici
Düzenleyicideki her renk düğmesi aynı seçiciyi açar. Doygunluğu ve parlaklığı seçmek için karede, tonu seçmek için yanındaki şeritte sürükleyin. Altta **R**, **G** ve **B** değerleri ile rengin opaklığı varsa bir **Opaklık** kaydırıcısı bulunur. Onaltılık alan `#RRGGBB` ya da `#RRGGBBAA` kabul eder; metin geçerli bir renk olmadığı sürece kırmızı görünür. **Kopyala** onaltılık kodu panoya kopyalar. Seçiciyi kapatmak için dışına tıklayın.

## Renk geçişleri
**Renk geçişi** ile Doğrusal renk geçişi ya da Radyal renk geçişi seçilir; yanındaki düğme renklerin sırasını tersine çevirir. Her durağın bir renk düğmesi ve bir **Konum (%)** alanı vardır; uçlar arasındaki durakların ayrıca bir kaldırma düğmesi bulunur. **Renk durağı ekle** en geniş boşluğa bir durak ekler; bir renk geçişinde en fazla sekiz durak olabilir. **Renk geçişi açısı** doğrusal bir renk geçişinin yönünü belirler; yanında **+45°** ve **−45°** düğmeleri vardır.

Önizlemenin altında her durak için bir işaretçi taşıyan bir ray bulunur. Durak eklemek için raya, seçicisini açmak için bir işaretçiye tıklayın; işaretçiyi taşımak için sürükleyin, kaldırmak için sağ tıklayın. Rayın altındaki üç örnek hazır ayarlardır: Sıcak renk geçişi, Soğuk renk geçişi ve Gri tonlu renk geçişi. Bir renk geçişinde yapılan değişiklikler seçime canlı olarak uygulanır.

Renk geçişinin yönü önemlidir: onu ilgisiz yüzeylerin üzerinden değil, bir çatı eğimi ya da kavisli bir nesne boyunca yönlendirin. Boyadıktan sonra modeli iki taraftan da kontrol edin.

## Yüzey malzemesi
**Yüzey malzemesi**; **Pürüzlülük** (0 parlak, 100 mat), **Metalik** (ahşap, taş ya da plastik için 0, metal için 100) ve **Işık yayımı** (0 ile 8 arası; yüzey renk geçişleri dahil kendi renginde parlar ama yakındaki nesneleri aydınlatmaz) değerlerini belirler. Değişiklikler seçili yüzeylere ya da fırçalara canlı olarak uygulanır. **Varsayılan yüzeyi kullan** bu ayarları kaldırır. Tüm değişiklikler geri alınabilir.

Yüzey efektleri aydınlatılmış 3B görünümde ve kamera işlemelerinde görünür. Düz ve tel kafes görüntüleme parlak vurguları bilerek dışarıda bırakır. Parlama (bloom), ışık yayan yüzeylerin çevresine bir hale ekleyebilir. Godot ve glTF/GLB malzeme değerlerini alır; Unity içe aktarıcısı ise pürüzlülüğü pürüzsüzlüğe (smoothness) çevirir. Her hedefin sınırları için `docs/surface-materials.md` dosyasına bakın.

# Hazır modeller, katmanlar ve gruplar
## Hazır model kitaplığı
**Öğeler** sekmesini açın. **Klasör ekle** ile `.map` modellerinden oluşan bir klasör ekleyin; yerleşik kitaplık her zaman oradadır. Aramak için **Öğeleri süz** alanına yazın. Bir modeli seçmek için tıklayın; ardından imlecin en son bulunduğu yere eklemek için çift tıklayın, `Enter` tuşuna basın ya da **Ekle** düğmesine tıklayın. Bir modeli bir görünüme sürükleyerek de ekleyebilirsiniz. Hazır modelin fırçaları tek bir grup olarak eklenir; böylece model bir bütün olarak hareket eder.

**Seçimi öğe olarak kaydet** seçili fırçaları kitaplığa bir `.map` olarak yazar. Kitaplıktaki modeller içe aktarılmış ağlar değil, sıradan ve düzenlenebilir fırçalardır.

## Katmanlar
Harita sekmesindeki **Katmanlar** listesi büyük alanları düzenlemeye yarar. Bir katman oluşturmak için bir ad yazıp **Ekle** düğmesine tıklayın ya da geçerli katmanı yeniden adlandırmak için `Enter` tuşuna basın. Her katman gizlenebilir, kilitlenebilir ya da dışa aktarımların dışında bırakılabilir. **Buraya taşı** seçimi geçerli katmana taşır. Görünümlerdeki bağlam menüsü de Katmana taşı, Etkin katman yap, Katmanları gizle ve Katmanları yalıt seçeneklerini sunar.

## Gruplar ve bağlantılı gruplar
`Ctrl+G` seçimi gruplar, `Ctrl+Shift+G` grubu çözer. Harita sekmesinin **Gruplar** bölümü grupları listeler ve **Aç**, **Yeniden adlandır**, **Birleştir**, **Grubu çöz**, **Çıkar** ve **Ayır** seçeneklerini sunar. Bağlantılı grup, düzenlemeleri paylaşan kopyalardan oluşan bir kümedir: `Ctrl+Shift+D` bağlantılı bir kopya oluşturur, Geometri → Bağlantılı grupları eşitle kopyaları günceller, **Bağlantıyı kaldır** ise bir kopyayı bağımsız hale getirir. Varlık sekmesinde bir özellik, bağlantılı gruplar eşitlendiğinde o kopyaya özgü kalacak şekilde ayarlanabilir.

## Ortak bir ölçek
Varsayılan dışa aktarma ölçeği metre başına 32 harita birimidir. Yerleşik oyuncu başlangıç işaretçisi 64 birim, yani 2 metre boyundadır ve bir başvuru ölçüsü olarak kullanılabilir. Kapı aralıklarını, araçları, mobilyaları ve bitkileri onunla karşılaştırın. Dışa aktarma ölçeğini yalnızca bilerek değiştirin ve ilişkili hazır modelleri tutarlı tutun.

# Varlıklar ve oynanış
Yaygın varlık tanımları yerleşiktir ve her zaman kullanılabilir: ışıklar, oyuncu başlangıç noktaları, kapılar, düğmeler, platformlar, tetikleyiciler, statik dekor nesneleri, sesler ve yol noktaları. Varlık sekmesindeki **Varlık tanımları** ile bir proje FGD'si yüklemek aynı adlı sınıfları değiştirir, diğer yerleşik sınıfları ise korur.

## Varlık yerleştirme
Varlık sekmesinin **Varlıklar** tarayıcısında **Sınıfları süz** alanına yazın ve bir sınıf seçin. Işık gibi bir nokta sınıfı için **Yerleştir** düğmesine tıklayın ya da sınıfa çift tıklayın; `Insert` seçimin merkezine bir tane yerleştirir. Kapı ya da tetikleyici gibi bir fırça sınıfı için fırçaları seçip **Oluştur** düğmesine tıklayın. Sınıflar bir görünüme sürüklenerek de yerleştirilebilir.

## Özellikler
Özellik listesi seçili varlığı düzenler. Aynı sınıftaki varlıklar birlikte düzenlenir. **Özellik ekle** tanımın bilmeyebileceği bir anahtar ekler; **Varsayılanlar** var olan özellikleri varsayılan değerlerine döndürebilir ya da eksik olanları ekleyebilir. Sınıfın tanımladığı ama varlıkta bulunmayan özellikler soluk gösterilir; bir değer yazdığınızda eklenirler. Varlıkları birbirine bağlamak için targetname ve target kullanın.

## Dışa aktarma neler yapabilir
Godot yönergeleri yerel düğümler, çarpışma gövdeleri ve tetikleme alanları oluşturur. Unity ve Unreal yaygın ışıkları yerel olarak destekler; Unreal ayrıca PlayerStart aktörleri oluşturur. Diğer veriler hedefin içe aktarıcısının izin verdiği ölçüde korunur.

Bir kapı varlığı tek başına oyun davranışı eklemez. Hareket, tetikleyici tepkileri, oyuncu denetleyicileri ve proje betikleri için motorda yine çalışmanız gerekir. Defold varlık özelliklerini ve çarpışma nesnelerini dışa aktarır; Flax modelleri ve genel varlıkları yerleştirir. Oynanışı üzerine kurmadan önce sonucu motorunuzda kontrol edin.

# Animasyon
Heron Brush basit hareketleri önizleyebilir, fırça varlıklarının anahtar kare animasyonlarını düzenleyebilir ve bunları birkaç motora aktarabilir.

## Animasyonları önizleme
Görünüm menüsünde, Animasyon menüsünde, Izgara ayarları menüsünde ya da Harita sekmesinin Görünüm bölümünde **Animasyonları önizle** seçeneğini açın. start_on etkin olduğunda varlığın axis, speed, direction, travel ve wait özelliklerini kullanarak func_rotate dönüşünü ve func_shuttle ileri-geri hareketini oynatır; Başlangıçta oynat olarak ayarlanmış anahtar kare kliplerini de oynatır. Vantilatör ve yel değirmeni hazır modelleri zaten dönen parçalar içerir.

Önizlemeyi kapatmak özgün pozu geri getirir. Bir görünüme tıklamak, seçme ya da düzenlemeden önce oynatmayı durdurur; kamera gezintisi çalışmaya devam eder. Önizleme haritayı, geri alma geçmişini ya da dışa aktarımları değiştirmez; fizik ya da oyun mantığı da çalıştırmaz.

## Anahtar kare düzenleyicisi
Düzenleyiciyi Animasyon → Anahtar kare animasyonu ile ya da Izgara ayarları menüsünden açın. Görünümlerin altına, 3B pencere ayrıyken de o pencereye yerleşir. Başlığındaki pencere simgesi düzenleyiciyi kendi penceresine taşır; geçerli kare ve geri alma geçmişi korunur. **Animasyonu düzenle** görünümleri animasyonlu poza geçirir, **Haritayı düzenle** ise dinlenme geometrisine geri döner. Animasyon kipinde her görünümün altında **Animasyonu düzenle** yazısı ve kare numarası görünür.

Canlandırmak için bir fırça varlığı ya da bir fırça veya fırça grubu seçin. Düz bir fırça grubu tek bir animasyonu ve dönme merkezini paylaşır; birden fazla varlık içeren gruplar için ayrı animasyonlar gerekir.

## Klipler
**Animasyon** seçicisi adlandırılmış klipler arasında geçiş yapar. Geçerli klibi yeni bir adla kopyalamak için **Animasyon adı** alanına adı yazıp **Farklı kaydet** düğmesine tıklayın; **Sil** bir klibi kaldırır. **Taşı** dönüş içermeyen bir `translate` klibi oluşturur. **Kapı animasyonları**, geçerli dönme merkezi etrafında 90 derecelik dönüşle yumuşak `open` ve `close` klipleri oluşturur; önce dönme merkezini menteşeye taşıyın. **Başlangıçta oynat** klibin seviye başladığında oynatılmasını sağlar; kapı hazır ayarı bu seçeneği kapatır.

## Oynatma denetimleri
Oynatma düğmeleri şunlardır: **Başlangıç**, **Önceki anahtar**, **Oynatma**, **Sonraki anahtar** ve **Son**. **Kare** geçerli kareyi gösterir ve ayarlar, **Süre (sn)** klibin uzunluğunu belirler. Kip **Bir kez**, **Döngü** ya da **İleri geri** olabilir.

## Zaman çizelgesi
Zaman çizelgesinde oynatma imlecini taşıyan bir cetvel ile **Konum** (X, Y ve Z birlikte anahtarlanır) ve her dönme açısı (RX, RY ve RZ) için birer şerit bulunur. Oynatma imlecini taşımak ve o pozu tüm görünümlerde önizlemek için cetvele tıklayın ya da cetvelde sürükleyin. Zaman çizelgesini kaydırmak için orta tuşla sürükleyin. Anahtarları zaman içinde taşımak için sürükleyin; `Shift` seçime ekler; anahtar eklemek için bir şeride çift tıklayın; `Ctrl`+tekerlek yakınlaştırır. **Eğriler** şeritler yerine değer eğrilerini gösterir.

Üstündeki düğmeler şunlardır: **Anahtar ekle**, **Kaldır**, **Kopyala**, **Yapıştır** ve **Çoğalt**. **Karelere yapıştır** anahtarları tam karelerde tutar, **Sığdır** tüm klibi gösterir. **Anahtar zamanlarını değiştir**; **Zaman ölçeği**, **Kaydırma (kare)**, **Oynatma imlecinden ölçekle**, **Süreyi uzat** ve **Seçimin zamanlarını değiştir** seçeneklerini açar.

Zaman çizelgesinin yanındaki anahtar denetçisi seçili anahtarı düzenler: **Kare**, **Değer** (ya da bir Konum anahtarının üç koordinatı), **Doğrusal**, **Sabit** ya da **Bézier** ara değerlemesi, **Giriş eğimi** ve **Çıkış eğimi** ile **Teğetleri yumuşat**.

## Düzenleyicideki tuşlar
Klavye zaman çizelgesindeyken: `Space` oynatır ya da duraklatır, sol ve sağ oklar birer kare ilerletir, `Ctrl+A` tüm anahtarları seçer, `Ctrl+C`, `Ctrl+X` ve `Ctrl+V` anahtarları kopyalar, keser ve yapıştırır, `Ctrl+D` çoğaltır, `Delete` kaldırır, `Ctrl+Z` geri alır ve `Esc` bir sürüklemeyi iptal eder. Zaman çizelgesinin dışında `Esc` animasyon kipinden çıkar. Geri alma; yakınlaştırmayı, seçimi ve oynatma imlecini oldukları yerde bırakır.

## Görünümlerde poz verme
Zaman çizelgesinin altındaki satırda şunlar bulunur: yakındaki pozları hayalet olarak gösteren, sayı ve **Aralık** ayarlı **Soğan zarı**; **Dönme merkezi** koordinatları; **Otomatik anahtar**; ve **Taşı**, **Döndür** ve **Dönme merkezi** poz araçları.

- **Anahtar ekle** Otomatik anahtar seçeneğini açar ve oynatmayı duraklatır. Otomatik anahtar açıkken ve **Taşı** seçiliyken, geçerli karede konumu anahtarlamak için 3B görünümde seçili varlığın tutamacındaki renkli eksenlerden birini sürükleyin.
- Bir 2B görünümde seçili fırçayı ya da grubunun bir üyesini sürükleyerek iki görünür eksen boyunca ötelemeyi ızgaraya tutturarak anahtarlayın. Bunun için Otomatik anahtar seçeneğinin açık olması gerekmez.
- **Döndür** ile fırçayı bir 2B görünümde turuncu dönme merkezi işaretçisinin etrafında sürükleyin. Üst görünüm RZ, Ön görünüm RY, Yan görünüm RX anahtarlar; açılar ızgara açılarına tutturulur.
- **Dönme merkezi** ile sarı dönme merkezini herhangi bir 2B görünümde sürükleyin. Merkez ızgaraya tutturulur ve derinliğini korur. Anahtarlar ve dinlenme geometrisi değişmez.

2B ve 3B görünümlerdeki camgöbeği yol, dönme merkezinin izlediği yolu gösterir. Zamanını ve anahtarlarını seçmek için bir yol noktasına tıklayın ya da konumu değiştirmek için noktayı sürükleyin; Otomatik anahtar açıkken yol noktalarını 3B görünümde de sürükleyebilirsiniz. Etiketler kare numaralarını gösterir, kesikli parçalar sıçramaları işaretler. `Esc` bu sürüklemelerin her birini iptal eder; tek bir geri alma tüm sürüklemeyi geri çevirir.

Yeni animasyonlar, sıfır anında anahtarlarla dinlenme pozunda başlar. Hareket oluşturmak için başka bir karede pozu değiştirin. Keskin, L biçimli bir yol için başlangıcı, köşeyi ve sonu anahtarlayın; Doğrusal ara değerlemeyi İleri geri kipiyle birlikte kullanın.

## Düğmeleri animasyonlara bağlama
Bir func_button, trigger_multiple ya da trigger_once seçin. Varlık sekmesi özelliklerin üstünde **Animasyon tetikleyicisi** bölümünü gösterir. Listeden bir hedef seçin ya da **Görünümde hedef seç** düğmesine, ardından herhangi bir görünümdeki animasyonlu bir nesneye tıklayın; `Esc` seçmeyi iptal eder. Eksik hedefler ya da klipler denetimlerin altında bildirilir. Bağlantı gösterimi açıkken varlık bağlantıları hedeflerini gösteren oklarla çizilir.

**Eylem**, etkinleştirmenin ne yapacağını seçer: **Oynat** adlandırılmış bir klibi ya da seçili animasyonu başlatır; **Geçiş yap**, varsa `open` ile `close` arasında gidip gelir, yoksa seçili animasyonu yeniden başlatır; **Aç** ve **Kapat** bu klipleri başlatır; **Duraklat** pozu tutar, **Sürdür** devam ettirir, **Durdur** ise özgün pozu geri getirir. **Yalnızca bir kez** sınıfın varsayılanını geçersiz kılar. **Yineleme aralığı (sn)** etkinleştirmeler arasındaki en kısa süreyi belirler; ilk etkinleştirmeyi geciktirmez.

**Tetikleyiciyi dene** eylemi, haritayı düzenlemeden ve geri alma adımı eklemeden önizlemede çalıştırır. **Önizlemeyi sıfırla** özgün pozları geri getirir, bir kerelik ve bekleme süresi durumlarını temizler. Bu işlemler oyuncu çarpışmalarını ya da oyun betiklerini simüle etmez.

## Dışa aktarımlarda animasyon
Godot dışa aktarımı, eğriler ve hareketli çarpışma dahil yerel AnimationPlayer ve AnimationLibrary kaynakları üretir. Bir düğmenin alanına giren bir CharacterBody3D, düğmenin hedefini etkinleştirir. Bu kliplerde Godot'ta yapılan düzenlemeler haritaya geri dönmez ve bir sonraki dışa aktarmada üzerine yazılır.

Raylib dışa aktarımı, C oynatıcısını içeren `<map>_animation.h` dosyasını ve serbest kameralı önizlemeyi içeren `<map>_preview.c` dosyasını ekler; derlemek için `<map>_raylib.md` dosyasındaki adımları izleyin. Önizlemede `SPACE` duraklatır, `R` varsayılan klipleri yeniden başlatır ve `N` adlandırılmış klipler arasında dolaşır. Defold, oynatmayı belgelenmiş `heron_*_animation` iletileriyle varlık betiğine gömer. Unity'deki Build Level, `HeronBrushAnimation` bileşenini ve kinematik bir gövde ekler. Bevy dışa aktarımı `heron_animation.rs` dosyasını ve talimatları içerir. Godot, Unity, Defold, Bevy ve raylib tetikleyici bağlantılarını, bir kerelik bayrağını ve yineleme aralığını korur. Oyununuzun yine de oynatıcıyı kaydetmesi ve kendi düğmelerini her çalışma zamanının API'sine bağlaması gerekir.

## Motion lab örneği
Karşılama ekranındaki Örnekler'den `motion_lab` haritasını açın. Haritada L biçimli bir yolda giden bir platform, bir asansör, dönen bir platform ve düğmeli, menteşeli bir kapı bulunur. Yanındaki `godot/project.godot` dosyasını Godot'ta açın ve `F5` tuşuna basın. WASD ve fare oyuncuyu hareket ettirir, Space zıplatır, 1 ile 4 arası tuşlar istasyonlara götürür, R ise geçerli istasyonu yeniden başlatır. Hedefini etkinleştirmek için renkli bir pedin üzerine basın. Birlikte gelen README yeniden dışa aktarmayı açıklar.

# Bir motora dışa aktarma
Dosya → Dışa aktar, Dışa aktar penceresini açar. Motorlar logolarıyla birlikte solda sıralanır: Godot, Defold, three.js, Bevy, raylib, Unity, Unreal Engine, Flax Engine, Cocos Creator, Wavefront .obj, glTF .gltf, GLB .glb ve en sonda `.map`. Sağdaki sayfa hedefin ne alacağını açıklar. **Yaz** dışa aktarımı yazar, **Denetle** seviyeyi yazmadan oluşturur ve sorunları listeler, **Kapat** ise pencereyi kapatır.

Dışa aktarma kaydetmeden ayrıdır: `.map` düzenlenebilir kaynağınız olarak kalır. Her denetim, değiştiği anda kendi motorunun ayarlarını kaydeder; böylece her motor kendi seçimlerini korur ve pencere bıraktığınız gibi açılır.

## Ortak seçenekler
- **Ölçek**: **Metre başına birim**, varsayılan olarak 32.
- **Yazılacaklar**: **Çarpışma geometrisi**.
- **İyileştirmeler**: **Statik ağları 1024 birimlik hücrelerde grupla**, **Geometrinin içine gömülü yüzeyleri kaldır**, **Aynı düzlemdeki yüzeyleri birleştir** ve **Çarpışma şekillerini birleştir**.
- **Proje yolları**: seviyenin ve kaynaklarının proje içinde nereye yazılacağı.
- **Araç malzemeleri**: clip, skip ve origin malzemelerinin adları.
- **Diğer**: **Düğümleri şuna göre adlandır**, **Katmanları ve grupları üst düğümler olarak yeniden oluştur**, **Üçgenlerin köşe sırasını ters çevir**, doku koordinatlarını çevirme ve **Işık haritası tekseli** ile pişirilmiş ışık için ikinci bir UV kümesi gibi motora özgü anahtarlar.
- **Haritayla eşitleniyor**: **Harita her kaydedildiğinde yeniden yaz**. Bu seçenek motora bir kez dışa aktardıktan sonra kullanılabilir hale gelir, çünkü nereye yazılacağını o dışa aktarma belirler.

Statik hücreler dünya fırçalarını merkezlerine göre atar ve her 1024 birimlik hücre içinde malzemeye göre birleştirir; böylece motorlar haritanın daha küçük parçalarını ayıklayabilir (culling). Çarpışma korunur ve oynanış varlıkları ayrı kalır. Tek bir dünya ağına göre daha fazla çizim çağrısı anlamına gelebileceği için varsayılan olarak kapalıdır. Tek başına OBJ, glTF ve GLB dışa aktarımları hücre kullanmaz.

## Hedeflere ilişkin notlar
- Godot: doku ve malzeme `res://` yolları, tetikleyici fizik katmanı ve ışık haritası UV2'si. Godot projesinin içinde bir dışa aktarma klasörü seçin.
- Defold: asset kaynak dizini, model malzemesi ve seçilen malzemede bulunması gereken doku örnekleyicisi.
- Unity: OBJ ağları ve bir düzenleyici içe aktarıcısı; projenin Assets klasörü altına dışa aktarın.
- Unreal Engine: oluşturulan bir içe aktarma betiği ve genellikle /Game/<level> olan bir içerik yolu.
- Flax Engine: projenin Content klasörüne göre bir model içerik yolu.
- three.js, Bevy, raylib ve Cocos Creator: bu çalışma zamanları için sahneler ya da kod; talimatlar dışa aktarımın yanında bulunur.
- glTF / GLB: uyumlu araçlar için taşınabilir geometri; GLB ikili biçimdir. OBJ: modelleme araçları için geometri ve malzemeler.
- `.map`: **Şu biçimde yaz** ile Valve 220, Standart, Quake 2 ya da Quake 2 (Valve) seçilir. Pencere, seçilen lehçede nelerin kaybolacağını belirtir.

Yazdıktan sonra eksik dokular gibi uyarıları okuyun. **Denetim sonuçları** bu uyarıları pencerede listeler.

# Canlı bağlantılar: Godot, Unity, Defold ve Flax
Canlı bağlantı, siz çalışırken Heron Brush'taki kaydedilmemiş düzenlemeleri bir motor düzenleyicisine gönderir. Tek yönlüdür: motorda yapılan düzenlemeler geri gelmez. Her harita penceresinde tek bir canlı bağlantı çalışabilir. Motor değiştirmeden önce bağlantıyı durdurun; değişen dışa aktarma seçeneklerini uygulamak için yeniden başlatın. Denetimler Dışa aktar penceresinde, motorun sayfasındaki canlı bağlantı kartındadır (örneğin **Godot ile canlı bağlantı**).

Başlık çubuğundaki gösterge bağlantının durumunu gösterir; bağlantıyı durdurmak için göstergeye tıklayın. Haritayı kapatmak ya da başka bir haritaya geçmek yayını durdurur. Bir içe aktarma hatasından sonra sorunu düzeltin ve bağlantıyı yeniden başlatın.

## Godot
1. Bir kez bir Godot projesine dışa aktarın.
2. **Godot eklentisini kur** düğmesine tıklayın.
3. Godot'ta Proje Ayarları → Eklentiler altında Heron Brush Live eklentisini etkinleştirin.
4. **Canlı bağlantıyı başlat** düğmesine tıklayın ve Godot panelinden haritayı açık bir 3B sahneye bağlayın.

Eklenti yalnızca kendi oluşturduğu dalı değiştirir; bağlama noktasının yerleşimini ve diğer düğümleri korur. Godot sahnesini kendiniz kaydedin; kendi betiklerinizi ve düğümlerinizi oluşturulan dalın dışında tutun.

## Unity
1. Bir kez Unity projesinin Assets klasörü altına dışa aktarın.
2. **Unity eklentisini kur** düğmesine tıklayın ve Unity'nin eklentiyi derlemesini bekleyin.
3. Unity'de Window → Heron Brush → Live Link'i açın.
4. Bağlantıyı başlatın ve haritayı etkin Unity sahnesine bağlayın.

Unity canlı penceresini açık tutun. Güncellemeler Play Mode ve Prefab Mode sırasında duraklar. Harita nesnesinin yalnızca Generated alt nesnesi değiştirilir. Oluşturulan dosyaları düzenlemeyin.

## Defold
1. Bir kez Defold projenize dışa aktarın ve projeyi Defold'da açık tutun.
2. **Canlı bağlantıyı başlat** düğmesine tıklayın. Eklenti gerekmez.
3. Defold'da heron_brush_live/preview.collection dosyasını açın ya da kendi sahnenizden ona başvurun.

Bağlantı ayrı, oluşturulmuş bir klasöre yazar; son dışa aktarıma ve game.project dosyasına dokunmaz. **Çalışan oyunu anında yeniden yükle**, Defold'dan başlatılan bir oyunu da günceller; nesne eklemek ya da çarpışmayı değiştirmek seviyenin yeniden başlatılmasını gerektirebilir. Bağlantı hataları otomatik olarak yeniden denenir.

## Flax
Projenin Content klasörüne dışa aktarın ve **Flax eklentisini kur** düğmesine tıklayın. Flax'ın eklentiyi derlemesini bekleyin, bağlantıyı başlatın, ardından açık Flax sahnesinde Tools → Heron Brush: attach live map komutunu seçin. Oluşturulan kök nesne yerleşimini korur, diğer nesnelere dokunulmaz. Sahneyi kendiniz kaydedin. Güncellemeler Play Mode sırasında duraklar. Tools → Heron Brush: stop live map alımı duraklatır.

# Ortak canlı oturumlar
İş birliği → Canlı oturum, başka bir Heron Brush kullanıcısı için oturum açmaya ya da onun oturumuna katılmaya yarayan bir pencere açar. Bu, harita düzenlemeyi kişiler arasında paylaşır; motor canlı bağlantısından ayrı bir özelliktir.

## Oturum açma ve katılma
**Adınız** alanını doldurun. Oturum açmak için **Oturum başlat** seçeneğini seçin, **Oturumu başlat** düğmesine tıklayın ve **Bağlantı bilgilerini kopyala** ile aldığınız bilgileri konuklarınıza gönderin. Katılmak için **Oturuma katıl** seçeneğini seçin, bilgileri yapıştırın ve **Katıl** düğmesine tıklayın. Katılmak açık haritanın yerine oturum sahibinin haritasını yükler; bu yüzden önce çalışmanızı kaydedin.

Standart sürüm yerel ağda çalışır; **Yerel ağ bağlantı noktası** kullanılacak bağlantı noktasını belirler. İnternet üzerinden oturumlar (**İnternet üzerinden bağlan**) isteğe bağlı ağ özelliğiyle derlenmiş bir sürüm gerektirir. Katılımcılar 3B görünümde birbirlerinin kamera işaretçilerini ve adlarını görür.

Pencereyi kapatmak oturumu sonlandırmaz; başlık çubuğundaki gösterge durumunu gösterir. **Oturumu durdur** ya da **Oturumdan ayrıl** oturumu bitirir. İki kişi aynı nesneyi düzenlediğinde son sürümü oturum sahibindeki sıralama belirler; geri alma da ortak bir düzenleme sayılır. Mümkün olduğunda ayrı alanlarda çalışın ve ara kayıtlar alın.

# 3B görüntüleme ve ikinci monitör
## Görüntüleme kipleri
Harita sekmesinin **Görünüm** bölümü görünümlerin neleri çizeceğini belirler: Fırçalar, Nokta varlıkları, Varlık modelleri, Varlık sınıf adları, Grup ana hatları, Yamalar, Malzemeler, Atlanan yüzleri göster, Animasyonları önizle, Tel kafes, Kenarlar ve Fırça varlığı ana hatları. Tel kafes; köşegenler ve yüzeylerin arkasında kalan kenarlar dahil üçgen kenarlarını malzemesiz çizer. Geçici bir hata ayıklama görünümüdür; dışa aktarımları ya da işlemeleri etkilemez.

3B görünümün aydınlatma, gölge, ortam gölgelemesi, yansıma, parlama ve yumuşatma ayarları Tercihler → İşleyici altındadır. Bunlar önizleme ayarlarıdır; son görüntüler işleme kamerasının kendi ayarlarını kullanır.

## Ayrı bir 3B penceresi
Görünüm → Ayrı pencerede 3B görünüm, 3B görünümü başka bir monitöre koyabileceğiniz ayrı bir pencereye taşır. Bu pencere kamerayı, belgeyi ve araçları paylaşır; ana pencerede ise düz görünümler yer kaplar. Menüsünde şunlar bulunur: Malzemeler, Katı görünüm ve Tel kafes; Atlanan yüzleri göster, Animasyonları önizle, Kenarlar, Grup ana hatları, Nokta varlıkları, Varlık modelleri ve Fırça varlığı ana hatları öğelerini içeren bir Görünüm alt menüsü; Seçime odaklan; kamerayı seçimin (ya da kameranın önündeki bir noktanın) etrafında döndüren Ön, Yan ve Üst; 3B görünümden kamera yerleştir; Tam ekran (`F11`); Görüntü işle; ve Ana pencereye dön. Pencereyi kapatmak görünümü ana pencereye geri yerleştirir.

## Yordamsal gökyüzü
Yordamsal gökyüzü 3B görünümde varsayılan olarak açıktır ve Tercihler → İşleyici → Işık altında kapatılabilir. İşleme kameralarının haritayla birlikte kaydedilen kendi **Yordamsal gökyüzü** anahtarı vardır. Gökyüzü, ufuk ve zemin renkleri ortam ayarlarından gelir, güneş ise sahne aydınlatmasını izler. Gökyüzünün geometrisi yoktur ve dışa aktarılmaz.

# İşleme kameraları ve PNG görüntüleri
Dosya → Görüntü işle, işleme penceresini açar: solda kamera ve ayarları, sağda siz değiştirdikçe yeniden çizilen canlı bir önizleme bulunur. Kameralar `.map` içinde, motor dışa aktarımlarının dışında bırakılan ve yalnızca düzenleyiciye özgü olan Render cameras katmanında kaydedilir.

## Kamerayı yerleştirme
Çekimi 3B görünümde çerçeveleyin ve `Ctrl+Alt+0` (ya da `Ctrl+Alt+Num0`) tuşuna basın veya **3B görünümden kamera yerleştir** düğmesine tıklayın. Hiçbir kamera seçili değilse kaydedilen ilk kamera güncellenir; hiç kamera yoksa yeni bir kamera oluşturulur. Bu işlem geri alınabilir ve çıktı ile aydınlatma ayarlarını korur. **Haritada kamerayı seç**, **Kamerayı haritadan yükle** ve **3B görünümü kameraya taşı** ters yönde çalışır. **3B görünümde kamera çerçevesi** çıktı sınırını gösterir, **Kamerayı 3B görünümden denetle** ise normal gezinmenin kaydedilen kamerayı hareket ettirmesini sağlar.

## Ayarlar
- **Kamera konumu**: X, Y, Z, sapma (yaw) ve yunuslama (pitch).
- **Çıktı ve lens**: **Ön ayarlar** (Full HD, Kare, Dikey), genişlik, yükseklik, örnek sayısı, **Dikey görüş açısı** ve **Uzak kırpma mesafesi**.
- **Aydınlatma**: Gün ışığı, Gün batımı ya da Kapalı hava; ya da pozlamayı, güneşi, gökyüzü, ufuk ve zemin renklerini, **Yordamsal gökyüzü**, **Gölgeler**, **Ortam gölgelemesi** ve **Parlama** ayarlarını kendiniz belirleyin.
- **Alan derinliği**: **Alan derinliğini etkinleştir**, odak uzaklığı, net aralık ve bulanıklık yarıçapı. **Seçime odaklan**, kameranın önündeki seçili bir nesnenin merkezine odaklanır.

**Kamerayı ve ayarları kaydet** formdaki değerleri haritaya kaydeder; diskte saklamak için haritayı da kaydedin.

## İşleme
**PNG işle** nereye kaydedileceğini sorar ve önizlemede örnek örnek dolan aşamalı bir işleme başlatır. Daha fazla örnek daha iyi görünür ama daha uzun sürer. **İşlemeyi iptal et** dosya yazmadan durdurur. İşleyici ışın izleme (path tracing) değil, düzenleyicinin rasterleştirme hattını kullanır. PNG'nin arka planı opaktır ve animasyonlar işlenmez. Çok büyük görüntüler düzenleyiciyi kısa bir süre duraklatabilir.

# Tercihler
Dosya → Tercihler (`Ctrl+,`) ayarları açar. Değişiklikler hemen uygulanır ve kaydedilir. Kenar çubuğunda Genel bakış, Oyunlar, Görünüm, Renkler, Fare, Klavye ve İşleyici sayfaları listelenir. Üstteki arama alanı, herhangi bir sayfadaki ayarı adına ya da açıklamasına göre bulur. Değiştirilen her ayarın varsayılan değerini geri yükleyen kendi düğmesi vardır; **Varsayılanları geri yükle** ise geçerli sayfayı sıfırlar.

## Sayfalar
- **Genel bakış**: sık kullanılan ayarlar bir arada: Görünüş (Panel opaklığı (%), Renk şeması, Yerleşim, Dil), Düzenleme, Görünüm ve Kılavuz.
- **Oyunlar**: oyun, dosyalarının yeri ve iş birliği adınız. Oyun; hangi harita biçiminin yazılacağını, hangi varlık tanımlarının yükleneceğini ve malzemelerinin nerede olduğunu belirler. Oyun yapılandırmaları, yapılandırma klasöründeki `games` klasöründen (Linux ve macOS'ta `~/.config/heron-brush/games`, Windows'ta `%LOCALAPPDATA%\heron-brush\games`) ve uygulamanın yanındaki `games` klasöründen okunur; paketler birlikte gelen yapılandırmayı bu ikinci klasöre koyar.
- **Görünüm**: Yerleşim, Renk şeması, Dil, dünya eksenleri, ızgara opaklığı, sızıntı denetimi hücre boyutu ve eşleme kilitleri.
- **Renkler**: görünümlerin, geometrinin ve tutamaçların renkleri.
- **Fare**: bakma, kaydırma ve hareket hızları ile ters çevirmeler, uçuş hızı, görüş alanı, 2B görünümleri bağla ve tutamaç boyutları.
- **Klavye**: tuş eşlemesi, Heron Brush ya da Blender.
- **İşleyici**: gerçekçi gölgelendirme, seçim ana hattı, ışık, gölgeler, ortam gölgelemesi, yansımalar, parlama ve yüzeyler.

## Dil
**Dil**, Masaüstünü izle seçeneğini ve çevirisi bulunan tüm dilleri sunar. Arayüz hemen değişir. Kılavuz, çevirisi varsa arayüz dilinde, yoksa İngilizce açılır.

# Eklentiler ve MCP
## Eklentiler
Eklentiler düzenleyiciyi Rhai betikleriyle genişletir. Dosya → Eklentiler, Eklentiler penceresini açar. **Eklenti kur** bir `.rhai` dosyasını, **Eklenti klasörü kur** ise `main.rhai` içeren bir klasörü kurar; isteğe bağlı `plugin.toml` meta verileri eklentiyi ve ayarlarını tanımlar. Eklentiler **Etkin eklentiler** ve **Etkin olmayan eklentiler** altında gruplanır; **Eklentiyi etkinleştir** bir eklentiyi açar ya da kapatır ve bu ayar yeniden başlatmadan sonra da korunur. **Eklentileri yeniden yükle** betikleri diskten yeniden okur ve panel değerlerini sıfırlar.

Etkin eklentiler ayarları, eylem düğmeleri (**Çalıştır**) ve çıktılarıyla birlikte Harita sekmesinin **Eklentiler** bölümünde de görünür. Bir eylem seçim üzerinde, hiçbir şey seçili değilse her şey üzerinde çalışır ve tek bir geri alma adımıdır. Birlikte gelen Selection inspector eklentisi sayıları ve yukarı ya da aşağı taşıma eylemlerini gösterir.

## MCP
İş birliği → Ajan için sunucu başlat (MCP), bir yapay zekâ ajanının ya da başka bir aracın açık düzenleyiciyi Model Context Protocol üzerinden yönetmesini sağlar. Pencere yerel adresi bir **Kopyala** düğmesiyle gösterir; **Sunucuyu durdur** sunucuyu kapatır. Sunucu çalışırken başlık çubuğunda **MCP etkin** görünür. `--mcp` komut satırı seçeneği aynı arayüzü penceresiz çalıştırır. İstemciler geometri, hazır modeller, özellikler, animasyon, dışa aktarma ve oturumlar için araçları keşfedebilir. MCP ve düzenleyici eklentileri ayrı arayüzlerdir.

# Komut satırı
Programın adı `heron-brush`. Seçenek verilmezse karşılama ekranını açar.

## Dosya açma
`heron-brush map.map [entities.fgd] [materials/]` bir haritayı, isteğe bağlı olarak varlık tanımları ve bir malzeme klasörüyle birlikte açar. Boş bir argüman o öğeyi atlar; örneğin `heron-brush map.map "" materials/`. Bir düzenleyici zaten çalışıyorsa harita onda açılır; `--new-instance` ise bunun yerine ayrı bir pencere açar.

## Komutlar
- `--help` ya da `-h`: kullanım bilgisini yazdır. `--version` ya da `-V`: sürümü yazdır.
- `--mcp`: MCP'yi penceresiz olarak standart giriş ve çıkış üzerinden sun.
- `--merge-map <base> <ours> <theirs> [<output>]`: git için `.map` dosyalarının üç yönlü birleştirmesi. Çıktı varsayılan olarak ours dosyasıdır. Çakışmalar yazdırılır ve sıfırdan farklı bir çıkış kodu döndürülür.
- `--obj-to-map <in.obj> <out.map> [size] [thickness] [snap] [material]`: bir ağı fırçalara dönüştür. size, harita birimi cinsinden en uzun kenarın uzunluğudur (varsayılan 256); snap köşeler için bir ızgaradır (kapalı için 0); `solid` ya da `layered` sözcükleri ince kamalar yerine ızgara kutuları ya da dışbükey dilimler oluşturur.
- `--render-preview <in.map> <out.png> [size]`: bir haritanın PNG küçük resmini yaz, varsayılan olarak 512 piksel.
- `--run-script <script.rhai> <in.map> [out.map] [key=value ...]`: bir eklenti betiğini her şey seçiliyken bir harita üzerinde çalıştır ve sonucu yaz (varsayılan olarak girdi dosyasının üzerine). `key=value` eklentinin ayarlarına değer verir.

# Sorun giderme
## Bir şey yanlış göründüğünde
- Eksik malzeme: malzeme klasörünü ya da paketini ve dışa aktarımın proje yollarını kontrol edin.
- Yanlış model boyutu: 64 birimlik oyuncu işaretçisiyle ve dışa aktarma ölçeğiyle karşılaştırın.
- Yanlış renk geçişi yönü: renk geçişi açısını ve yüzeyin yönelimini kontrol edin.
- Boş motor sahnesi: dışa aktarma uyarılarını okuyun ve hedefin içe aktarma talimatlarını izleyin.
- Canlı bağlantı bekliyor: eklentinin kurulu, sahnenin bağlı ve güncellemelerin duraklatılmamış olduğunu kontrol edin.
- Yalnızca çizgiler var, yüzey yok: Harita sekmesinin Görünüm bölümünde Tel kafes seçeneğini kapatın ya da 3B penceresinin menüsünde Malzemeler veya Katı görünüm seçeneğini seçin.
- Görünümlerde eksik nesneler: `Ctrl+Shift+I` gizli nesneleri gösterir; katmanları ve Görünüm bölümündeki filtreleri kontrol edin.
- Bir tuş hiçbir şey yapmıyor: klavyenin görünüme geçmesi için önce görünüme tıklayın ve Tercihler → Klavye'de tuş eşlemesini kontrol edin.
- Bir düzenleme başarısız oldu: geri alın, geometriyi sadeleştirin ve Harita sekmesinin **Sorunlar** bölümündeki **Haritayı denetle** ile **Sızıntıları denetle** düğmelerini kullanın.
- Bir harita dosya yöneticisinden açılmıyor: zaten çalışan bir düzenleyiciye aktarılmış olabilir; o pencereye bakın.

## Ayarlar ve yedekler
Tercihler, son haritalar, yıldızlar, dışa aktarma ayarları ve eklentiler yapılandırma klasöründe tutulur: Linux ve macOS'ta `~/.config/heron-brush`, Windows'ta `%LOCALAPPDATA%\heron-brush`. Otomatik kayıt yedekleri her haritanın yanındaki `autosave` klasöründedir.

## Hata bildirme
Düzenleyici sürümünü (Yardım → Hakkında), işletim sisteminizi, sorunu gösteren küçük bir haritayı ve izlediğiniz adımları tam olarak ekleyin. Çıktı bölmesi (`Ctrl+4`) çoğu zaman neyin yanlış gittiğini söyler. Denemelere başlamadan önce bir kopya kaydedin.

## Bu kılavuzu kullanma
Soldaki içindekiler listesinden bir bölüm seçin. `Ctrl+F` içindekileri gösterir ve arama alanına geçer; `Esc` aramayı temizler, arama boşsa kılavuzu kapatır. **İçindekiler** bölüm listesini gizler ya da gösterir, **Metin boyutu** yazı boyutunu değiştirir; ikisi de hatırlanır. Üstteki dil listesi mevcut çeviriler arasında geçiş yapar.
