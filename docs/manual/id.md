# Memulai
Heron Brush adalah editor level berbasis brush. Anda membangun level dari brush cembung, mewarnai dan memberi tekstur pada permukaannya, menempatkan entitas, menganimasikan pintu dan platform, lalu mengekspor hasilnya ke mesin game. Tersedia pula aset bawaan, bentuk parametris, tautan langsung ke editor mesin game, sesi pengeditan bersama, dan render gambar diam. Panduan ini disertakan bersama aplikasi dan dapat dibuka tanpa internet.

Di macOS, `Cmd` berfungsi di mana pun panduan ini menyebut `Ctrl`, dan Option sama dengan Alt.

## Layar sambutan
Saat Heron Brush dimulai tanpa peta, layar sambutan terbuka. Di sebelah kiri ada **Lanjutkan** (peta terakhir yang Anda buka), **Peta baru** dan **Telusuri** untuk membuka `.map` yang sudah ada, serta **Game**, **Panduan**, dan **Preferensi**. Di sebelah kanan ada daftar peta terbaru beserta gambar mininya, diikuti **Contoh**. Bintang (**Selalu di atas**) menyematkan peta di bagian atas daftar; tempat sampah (**Pindahkan ke tempat sampah**) memindahkan berkas ke tempat sampah sistem sehingga masih dapat dipulihkan. Tombol panah berpindah antarelemen layar dan Enter membuka item yang sedang difokuskan. Berkas → Selamat datang menampilkan layar ini lagi kapan saja.

## Peta pertama Anda
1. Pilih **Peta baru** di layar sambutan, atau Berkas → Peta baru (`Ctrl+N`).
2. Pilih alat Brush (`B`) di bilah alat lalu seret di tampilan 2D untuk menggambar bentuk yang dipilih. Periksa kedalamannya di tampilan 2D lain.
3. Pilih brush tersebut dan pindahkan dengan menyeretnya. Gunakan tab **Permukaan** untuk memberinya material atau warna.
4. Simpan peta sumber dengan `Ctrl+S`. Setelah siap dipakai di game, buka Berkas → Ekspor dan tulis scene untuk mesin game Anda.

## Berkas dan penyimpanan
Berkas `.map` adalah sumber yang dapat Anda edit. Simpan secara rutin: ekspor, tautan langsung, dan render tidak pernah menyimpan peta untuk Anda. Berkas → Simpan sebagai (`Ctrl+Shift+S`) menambahkan ekstensi `.map` jika Anda tidak menuliskannya. Berkas → Kembalikan ke peta tersimpan membuang perubahan yang belum disimpan dan memuat ulang berkas. Jika Anda menutup jendela atau membuka peta lain saat masih ada perubahan yang belum disimpan, Heron Brush bertanya apakah akan **Simpan**, **Buang**, atau **Batal**.

Selama peta yang sudah disimpan memiliki perubahan yang belum disimpan, cadangan bernomor ditulis setiap sepuluh menit ke folder `autosave` di samping peta. 50 cadangan terbaru dipertahankan.

Jika Anda membuka peta saat Heron Brush sudah berjalan, peta itu diserahkan ke jendela yang sedang berjalan, yang kemudian muncul ke depan. Gunakan `--new-instance` di baris perintah jika Anda menginginkan jendela terpisah (lihat Baris perintah).

# Antarmuka
Jendela utama memiliki bilah judul berisi menu, bilah alat di bawahnya, tampilan peta, inspektur di sebelah kanan, dan bilah status di bagian bawah.

## Bilah judul dan menu
Bilah judul menampilkan logo, menu, nama peta di tengah, dan tombol jendela. Di macOS, tombol merah, kuning, dan hijau yang biasa berada di kiri. Seret bilah judul untuk memindahkan jendela dan klik dua kali untuk memaksimalkannya. Selama ada proses yang berjalan di latar belakang, sebuah indikator muncul di dekat ujung bilah: tautan langsung ke mesin game, sesi langsung, atau **MCP aktif**. Klik indikator untuk menghentikan tautan langsung atau membuka jendelanya.

- **Berkas**: Selamat datang, Peta baru (`Ctrl+N`), Buka peta (`Ctrl+O`), Simpan (`Ctrl+S`), Simpan sebagai (`Ctrl+Shift+S`), Kembalikan ke peta tersimpan, Ekspor, Render gambar, Plugin, dan Preferensi (`Ctrl+,`).
- **Edit**: Batalkan (`Ctrl+Z`), Ulangi (`Ctrl+Shift+Z`), Potong, Salin, Tempel, Duplikasi (`Ctrl+D`), Hapus, Pilih semua (`Ctrl+A`), Ulangi perintah terakhir (`Ctrl+R`), dan Lupakan perintah yang dapat diulang.
- **Tampilan**: Tampilan 3D di jendela terpisah, Pergi ke, Lekatkan ke kisi (`Alt+0`), Tampilkan kisi (`0`), Pratinjau animasi, dan Tampilkan sisi yang diabaikan.
- **Geometri**: Sisipkan teks 3D, Sambungkan sudut, Ubah permukaan menjadi patch, Keluarkan pilihan dari entitasnya, dan Sinkronkan grup tertaut.
- **Animasi**: Animasi keyframe, Pratinjau animasi, dan Tempatkan kamera dari tampilan 3D (`Ctrl+Alt+0`).
- **Peta**: Muat berkas titik, Muat ulang berkas titik, dan Lepas berkas titik; Muat berkas portal, Muat ulang berkas portal, dan Lepas berkas portal. Berkas ini menampilkan jejak kebocoran dan portal yang ditulis oleh kompiler peta.
- **Kolaborasi**: Sesi langsung dan Jalankan server untuk agen (MCP).
- **Bantuan**: Panduan dan Tentang.

## Bilah alat
Bilah alat memuat semua alat. Kelompok pertama dipakai terus-menerus: Pilih, Brush, Potong, Verteks, Tepi, Permukaan, Putar, dan Skala. Kelompok kedua berisi Geser miring, Rakit, Sweep, Titik kendali, Pembulatan, dan Cat. Arahkan penunjuk ke sebuah tombol untuk melihat nama dan pintasannya. Mengklik alat yang sedang aktif sekali lagi akan kembali ke Pilih.

Setelah alat-alat itu ada **Balik horizontal** dan **Balik vertikal**, lalu dua sakelar: **Kunci** mempertahankan pemetaan material saat brush dipindahkan, dan **UV** mempertahankannya saat brush diputar (`U` menyalakan dan mematikannya). Berikutnya pengaturan khusus alat yang aktif: alat Brush menampilkan bentuk dan opsinya, sedangkan alat Pembulatan menampilkan radius, jumlah permukaan, dan sudutnya beserta **Terapkan**. Di ujung kanan ada nama alat, ukuran kisi (**Kisi 16** dan seterusnya; klik untuk memilih ukuran lain), dan menu **Pengaturan kisi** berisi Lekatkan ke kisi, Tampilkan kisi, Pratinjau animasi, Tampilkan sisi yang diabaikan, dan Animasi keyframe.

## Tampilan peta
Ada empat tampilan: tampilan 3D dan tiga tampilan datar, Atas (XY), Depan (XZ), dan Samping (YZ). Preferensi → Tampilan → Tata letak memilih Satu panel, Dua panel, Tiga panel, atau Empat panel. Seret pembatas untuk mengubah ukuran panel. Mengklik sebuah tampilan memberinya fokus keyboard. `Space` mengganti panel yang menampilkan satu tampilan sekaligus ke tampilan berikutnya, dan `Ctrl+Space` memaksimalkan tampilan yang memegang fokus keyboard lalu memulihkan tata letak lagi.

Klik kanan tanpa menyeret di tampilan mana pun untuk membuka menu konteks bagi pilihan dan objek di bawah penunjuk. Menu ini menyediakan pengelompokan, lapisan, grup tertaut, pembuatan entitas, pemindahan brush ke dalam atau ke luar entitas, penyalinan nama material, serta menampilkan material itu di peramban material.

## Inspektur
Inspektur di sebelah kanan memiliki empat tab. Seret tepi kirinya untuk mengubah lebarnya.

- **Peta** (`Ctrl+1`): Lapisan, Grup, Plugin, Masalah, filter di bagian Tampilan, dan Tag.
- **Entitas** (`Ctrl+2`): peramban entitas, definisi entitas, properti entitas yang dipilih, dan, untuk tombol serta pemicu, Pemicu animasi.
- **Permukaan** (`Ctrl+3`): UV, Material, Warna, Sweep, Pembulatan, Patch, dan Inset dan bevel. Setiap bagian dapat dilipat.
- **Aset**: pustaka model brush yang dapat dipakai ulang.

`Ctrl+5` menyembunyikan atau menampilkan inspektur dan `Ctrl+Alt+T` menyembunyikan atau menampilkan bilah alat, sehingga tampilan mendapat ruang lebih luas.

## Bilah status dan panel keluaran
Bilah status di bagian bawah menampilkan pesan terbaru dari editor. Kesalahan ditampilkan dengan warna merah. Klik bilah tersebut, atau tekan `Ctrl+4`, untuk membuka panel keluaran berisi riwayat lengkap pesan. Sebagian besar perintah melaporkan hasilnya di sini, misalnya berapa banyak brush yang terpotong.

## Dialog dan penampilan
Dialog seperti Preferensi, Ekspor, dan Render gambar terbuka di jendelanya sendiri. Preferensi → Tampilan → Skema warna memilih Ikuti sistem, Terang, atau Gelap. **Opasitas panel (%)** di halaman Ringkasan membuat panel tembus pandang di atas tampilan. Jendela menggambar tombolnya sendiri sesuai gaya sistem Anda: native di macOS, gaya Windows 11 di Windows, dan gaya GNOME di Linux.

# Navigasi dan keyboard
## Bergerak di tampilan 3D
Tahan tombol kanan mouse dan seret untuk melihat sekeliling. Seret dengan tombol tengah untuk menggeser pandangan. Roda mouse bergerak maju dan mundur. `W`, `A`, `S`, dan `D` terbang maju, ke kiri, mundur, dan ke kanan, `Q` naik dan `X` turun; tahan `Shift` untuk langkah yang lebih besar. Jika **Alt + seret tombol tengah menggerakkan kamera** diaktifkan di Preferensi → Mouse, Alt dengan tombol tengah bergerak searah pandangan. Kecepatan, pembalikan arah, dan bidang pandang ada di Preferensi → Mouse.

## Bergerak di tampilan 2D
Seret dengan tombol tengah untuk menggeser pandangan dan gunakan roda mouse untuk memperbesar atau memperkecil di sekitar penunjuk. Menggeser dan memperbesar satu tampilan datar menjaga tampilan lainnya tetap sejajar; **Tautkan tampilan 2D** di Preferensi → Mouse mengatur hal ini. `Ctrl+Alt+Z` mengatur ulang zoom dan `Ctrl+Shift+U` mengembalikan tampilan datar ke titik asal.

`Ctrl+U` membingkai pilihan di setiap tampilan. Tampilan → Pergi ke (`Ctrl+Alt+M`) memindahkan kamera ke koordinat yang diketik, memilih objek pada baris tertentu di berkas peta, atau memindahkan pilihan sejauh nilai yang diketik.

## Kisi dan pelekatan
`+` dan `-` mengubah ukuran kisi, dan tombol angka `1` hingga `9` langsung memilih ukuran, dari 1 hingga 256 unit. `0` atau `#` menampilkan atau menyembunyikan kisi, dan `Alt+0` menyalakan atau mematikan pelekatan. `S` melekatkan verteks brush yang dipilih ke kisi saat ini, `Ctrl+Shift+V` melekatkannya ke unit bulat, dan `Ctrl+Alt+Shift+V` ke kisi.

## Pintasan umum
- `Ctrl+S`: simpan peta. `Ctrl+Shift+S`: simpan sebagai.
- `Ctrl+Z`: batalkan. `Ctrl+Shift+Z` atau `Ctrl+Y`: ulangi.
- `Ctrl+C`, `Ctrl+X`, `Ctrl+V`: salin, potong, dan tempel. Tempel meletakkan objek di posisi penunjuk; `Ctrl+Alt+V` mempertahankan koordinat aslinya. Jika permukaan yang dipilih, salin mengambil material dan pengaturan UV-nya.
- `Ctrl+D`: duplikasi. `Ctrl+Shift+D`: duplikat tertaut dari sebuah grup.
- `Delete` atau `Backspace`: hapus pilihan.
- `Esc`: membatalkan seretan saat ini, mundur satu langkah di alat yang aktif, kembali ke Pilih, dan akhirnya mengosongkan pilihan. `Shift+Esc` langsung kembali ke Pilih.
- `Ctrl+R`: ulangi perintah terakhir yang dapat diulang. `Ctrl+Shift+R`: lupakan perintah tersebut.
- `Ctrl+Alt+0` atau `Ctrl+Alt+Num0`: tempatkan kamera render di posisi tampilan 3D.
- `F5`: muat ulang material. `F6`: muat ulang definisi entitas.

## Pintasan pemilihan
- `Ctrl+A`: pilih semua, atau semua handle saat alat verteks, tepi, atau permukaan aktif. `Ctrl+Shift+A`: kosongkan pilihan. `Ctrl+Alt+A`: balikkan pilihan.
- `Ctrl+B`: pilih objek lain dalam entitas atau grup yang sama.
- `Ctrl+T`: pilih semua yang disentuh brush terpilih. `Ctrl+E`: pilih semua yang ada di dalamnya. `Ctrl+Shift+E`: pilih semua yang berada dalam kolomnya sepanjang sumbu pandang. Perintah-perintah ini menghabiskan brush yang dipakai untuk memilih.
- `Ctrl+I`: isolasi pilihan. `Ctrl+Alt+I`: sembunyikan pilihan. `Ctrl+Shift+I`: tampilkan semuanya lagi.
- `Ctrl+G`: kelompokkan. `Ctrl+Shift+G`: bubarkan grup.

## Pintasan transformasi
- Tombol panah serta `Page Up` / `Page Down` memindahkan pilihan satu langkah kisi ke arah yang ditunjuknya di tampilan aktif. Dengan `Ctrl` pilihan diduplikasi sambil dipindahkan, dan dengan `Alt` pilihan diputar.
- `Alt+X`, `Alt+Y`, `Alt+Z`: putar seperempat putaran pada sumbu tersebut; tambahkan `Shift` untuk arah sebaliknya.
- `Ctrl+F`: balik melintang tampilan; `Ctrl+Alt+F`: balik atas-bawah. `Ctrl+Shift+X` dan `Ctrl+Shift+Y` mencerminkan sepanjang X atau Y.
- `Insert`: tempatkan entitas titik di tengah pilihan.
- `Tab`: saat brush dipilih, berpindah berurutan Pilih → Verteks → Tepi → Permukaan → Pilih.

## Pemetaan tombol Blender
Preferensi → Keyboard → Pemetaan tombol menyediakan Heron Brush (bawaan, dipakai di seluruh panduan ini) dan Blender. Dengan Blender: `G` memilih dan memindahkan, `R` memutar, `S` menskalakan, `E` melakukan sweep, `1`, `2`, dan `3` memilih alat verteks, tepi, dan permukaan, `K` memotong, `B` memilih, `A` memilih semuanya dan `Alt+A` melepas pilihan, `X` menghapus, `Shift+D` menduplikasi, `H` menyembunyikan dan `Alt+H` menampilkan semuanya, serta `Ctrl+Shift+Z` mengulangi. Selebihnya sama pada kedua pemetaan.

# Brush dan alat pemodelan
Brush adalah benda padat cembung yang didefinisikan oleh bidang-bidang. Bangun objek yang rumit dari beberapa brush sederhana alih-alih mencoba membuat satu brush menjadi cekung. Jaga jumlah brush tetap sedikit jika bentuk sederhana sudah cukup.

## Menggambar brush
Pilih alat Brush (`B`) lalu seret di sebuah tampilan. Bilah alat menampilkan **Bentuk yang digambar**: Balok, Tangga, Lengkung, Silinder, Kerucut, Sferoid (UV), Sferoid (ikosahedron), dan Sisipkan teks 3D. `Alt+1` hingga `Alt+7` langsung memilih bentuk. Bergantung pada bentuknya, bilah menampilkan **Buat berongga** dan **Ketebalan:**, **Tinggi anak tangga:** dan **Arah:** untuk tangga, **Sumbu:**, **Sisi:** dan mode lingkaran untuk bentuk bulat, **Cincin:** untuk bola, serta **Ketelitian:** untuk ikosahedron. Atur opsinya sebelum menyeret. Periksa hasilnya di lebih dari satu tampilan.

## Memilih
Klik brush untuk memilihnya, dan klik dengan `Ctrl` untuk menambahkannya ke pilihan. Menyeret dari ruang kosong menggambar kotak pilihan. Di tampilan 3D, klik dengan `Shift` memilih satu permukaan dan klik dengan `Ctrl+Shift` menambahkan permukaan lain. Menyeret brush yang dipilih akan memindahkannya; menyeret brush yang tidak dipilih akan memilih dan memindahkan brush itu saja.

Di tampilan 3D, seret permukaan brush yang dipilih sambil menahan `Shift` untuk mengekstrusinya. Klik permukaan lain sambil menahan `Alt` untuk menyalin material dan perataan permukaan yang dipilih ke permukaan tersebut; `Alt+Ctrl` hanya menyalin material dan `Alt+Shift` menyalin rotasinya.

## Alat pembentuk
- **Pilih** (`Esc`): memilih dan memindahkan.
- **Verteks** (`V`), **Tepi** (`E`), **Permukaan** (`F`): menyeret sudut, tepi, atau permukaan. Menyeret dengan `Shift` dari tengah tepi atau permukaan menambahkan sudut di sana. `Delete` menghapus sudut yang dipilih.
- **Putar** (`R`), **Skala** (`T`), **Geser miring** (`G`): mentransformasi pilihan dengan handle.
- **Potong** (`C`): klik dua atau tiga titik untuk menentukan bidang potong. `Enter` memotong, `Ctrl+Enter` memilih sisi yang dipertahankan, dan `Backspace` atau `Esc` menghapus titik terakhir.
- **Rakit** (`N`): tempatkan titik-titik di sekitar sebuah volume dan tekan `Enter` untuk membuat brush cembung darinya. `Backspace` menghapus titik terakhir.
- **Sweep** (`Y`): lihat bagian Sweep di bawah.
- **Titik kendali** (`P`): membentuk ulang kisi kendali sebuah patch.
- **Pembulatan** (`O`): klik sebuah tepi untuk membulatkannya, atau pilih tepi atau verteks dengan alatnya lalu tekan `Enter`. Semakin banyak permukaan, semakin halus hasilnya dan semakin banyak pula geometrinya.
- **Cat** (`K`): lihat Material dan UV.

## Menggabungkan brush
- `Ctrl+J`: gabungkan brush yang dipilih menjadi selubung cembungnya (convex hull).
- `Ctrl+K`: kurangkan brush yang dipilih dari brush yang disentuhnya.
- `Ctrl+Shift+K`: jadikan pilihan berongga, dengan dinding setebal satu langkah kisi.
- `Ctrl+L`: ambil irisan pilihan.

## Sweep, pembulatan, patch, dan bevel
**Sweep** membangun geometri di sepanjang lintasan dari permukaan yang dipilih. Pilih permukaannya (di alat Sweep, klik sambil menahan `Alt`), lalu atur **Perpindahan**, **Putaran** (sumbu dan sudut), **Skala**, **Langkah**, dan **Lintasan** (perataan Bilangan bulat atau Bebas; Busur, Lurus, atau Lengkung S) di bagian Sweep pada tab Permukaan, lalu tekan **Sweep**. Baris di bawah kontrol menyebutkan berapa banyak brush yang akan dibuat.

Bagian **Pembulatan** membulatkan tepi yang dipilih dengan **Radius**, **Permukaan**, dan **Sudut** (Serong atau Dibulatkan); satu permukaan menghasilkan tepi serong (chamfer).

**Patch** menampilkan **Kolom** dan **Baris** dari patch yang dipilih, dengan tombol untuk menambah atau menghapus sepasang. Geometri → Ubah permukaan menjadi patch (`Ctrl+P`) membuat patch dari permukaan yang dipilih.

**Inset dan bevel** bekerja pada permukaan yang dipilih: **Inset** menarik permukaan ke dalam dari tepinya dan **Kedalaman** mendorongnya keluar (atau ke dalam, jika negatif). **Permukaan bevel** menerapkan keduanya; tombol ketiga menambahkan panel atau membuat ceruk, bergantung pada tanda Kedalaman. **Sambungkan sudut** membuka dialog yang menyambungkan dua brush pada sudut serong atau bulat, lengkap dengan pratinjau.

## Teks 3D
Geometri → Sisipkan teks 3D (juga entri terakhir di daftar bentuk alat Brush) membangun brush dari teks. Fitur ini memakai Heron Block, fon poligon bawaan dengan sudut terpotong, lebar proporsional, dan aksen Ceko; huruf kecil memakai bentuk kapital. Ketik **Teks**, atur **Tinggi** (tinggi huruf kapital; aksen menjulang di atasnya) dan **Kedalaman**, perhatikan pratinjau langsungnya, lalu tekan **Sisipkan**.

Jika suatu operasi tidak dapat menghasilkan geometri yang valid, sesuaikan dimensinya, atau batalkan lalu sederhanakan bentuknya.

## Larik
Geometri → Larik menyalin pilihan dan menampilkan salinan di peta selagi Anda mengubah nilai. Jumlah sudah termasuk aslinya, dan larik membuat paling banyak 1024 salinan.

- **Deret** menggeser setiap salinan dari salinan sebelumnya sebesar **Langkah X**, **Langkah Y**, dan **Langkah Z** dan memutarnya **Putaran per salinan (derajat)** pada sumbu yang dipilih.
- **Kisi** mengisi **Jumlah X**, **Jumlah Y**, dan **Jumlah Z** item dengan jarak **Langkah X**, **Langkah Y**, dan **Langkah Z**.
- **Cincin** menyebar item sepanjang **Sudut (derajat)** mengelilingi pusat yang berjarak **Radius (unit)** dari pilihan. Dengan **Hadap ke pusat**, setiap salinan ikut berputar bersama cincin.
- **Sebaran** menaruh salinan di tempat acak dalam radius, melintang terhadap sumbu yang dipilih. **Seed** yang sama memberi tata letak yang sama, dan **Putaran acak** memutar setiap salinan.

Bila pilihan adalah satu grup utuh, **Salinan tertaut** menjadikan setiap salinan duplikat tertaut, sehingga perubahan pada satu salinan sampai ke semuanya. **Terapkan** mempertahankan salinan dan memilihnya bersama aslinya; **Batal** menghapusnya. Seluruh larik dibatalkan dalam satu langkah.

## Memahat medan
Medan terdiri dari banyak brush kecil yang permukaannya didorong dan ditarik oleh kuas pahat. Pilih satu atau beberapa brush lalu pilih Geometri → Bagi menjadi kolom. Brush dipotong sepanjang garis kisi menjadi kolom selebar satu langkah kisi, dan kolom-kolom itu dimasukkan ke grup bernama Terrain. Potongan mengikuti kisi dunia, jadi brush yang bersebelahan terpotong di tempat yang sama dan berbagi sudut. Setiap kolom berdiri searah sisi brush yang paling tipis: tegak di lantai atau langit-langit, mendatar di dinding. Gua dibuat dengan cara yang sama: lubangi sebuah kotak (Hollow selection) dengan tebal dinding satu langkah kisi, pilih dindingnya, lalu bagi.

Jendela **Pahat**, yang dibuka tombol di ujung bilah pahat, memilih bentuk kolom dan juga bisa membagi pilihan. **Segitiga** membuat dua kolom per sel kisi, dan bagian atas segitiga tetap datar bagaimanapun dipahat. **Persegi** membuat satu kolom per sel, jadi jumlah brush hanya separuhnya; bagian atasnya menekuk di sepanjang diagonal. **Segi enam** menutupi brush dengan segi enam selebar dua langkah kisi, masing-masing dibagi menjadi enam segitiga di sekitar pusatnya, sehingga setiap bagian atas tetap datar.

Pilih alat **Pahat** (`J`) dan seret di atas permukaan pada tampilan 3D. Cincin oranye menunjukkan kuas. Tanpa pilihan, kuas mengubah setiap brush di bawahnya; dengan pilihan, hanya yang dipilih. Bilah di bagian atas jendela memilih apa yang dilakukan sapuan:

- **Naikkan** dan **Turunkan** menggerakkan permukaan ke luar atau ke dalam, paling banyak di tengah cincin dan sama sekali tidak di tepinya. Tahan `Shift` untuk menukarnya.
- **Haluskan** meratakan tonjolan.
- **Ratakan** meratakan permukaan ke tempat sapuan dimulai.

Di jendela yang sama, **Arah** memilih ke mana:

- **Atas** menggerakkan permukaan lurus ke atas dan ke bawah, di dalam lingkaran di tanah. Ini untuk medan.
- **Kamera** menggerakkannya mendekati dan menjauhi tampilan, di dalam bola di sekitar penunjuk. Arahkan pandangan ke lantai, dinding, atau langit-langit, dan bagian itulah yang dibentuk.
- **Normal** menggerakkannya keluar dari permukaan yang ada di bawah penunjuk saat sapuan dimulai.

Dua angka itu adalah **Radius (unit)** dan **Kekuatan (%)**. Hanya sudut di sisi yang digerakkan sapuan yang bergerak; sisi belakang tetap di tempat dan kolom tetap setebal paling sedikit satu unit. Sudut yang dipakai bersama oleh beberapa brush bergerak bersama, jadi permukaan tetap tertutup. Satu sapuan dibatalkan dalam satu langkah. Pada tampilan 2D alat ini memilih seperti alat Pilih.

Setelah selesai memahat, pilih medan lalu pilih Geometri → **Gabungkan kolom**, atau tombol di jendela **Pahat**. Kolom bersebelahan yang bersama-sama membentuk brush cembung dan tampak sama menjadi satu brush, sehingga bagian datar menjadi beberapa brush besar. Permukaannya tidak bergeser, dan penggabungan bisa dibatalkan dalam satu langkah.

# Material dan UV
## Memuat dan menerapkan material
Buka bagian **Material** di tab Permukaan. **Muat folder** menambahkan folder gambar dan **Muat .wad** menambahkan paket tekstur; keduanya muncul di bawah **Koleksi**, dan tanda silang menghapusnya. Ketik di **Filter material** untuk mempersempit daftar. Klik sebuah material untuk menjadikannya material aktif, lalu **Terapkan pada pilihan** memasangnya di setiap permukaan yang dipilih. **Pilih** menyediakan Pilih brush dengan material ini dan Pilih permukaan dengan material ini. **Ganti** menukar satu material dengan material lain di seluruh peta atau hanya di dalam pilihan.

Alat **Cat** (`K`) memasang material aktif pada permukaan di bawah penunjuk: satu klik mengecat satu permukaan, klik dua kali mengecat seluruh brush, dan klik sambil menahan `Shift` mengambil material permukaan itu sebagai material aktif, seperti pipet.

## Editor UV
Bagian **UV** di tab Permukaan menampilkan material dan garis luar permukaan yang dipilih. Kontrol mouse di kanvas:
- Roda mouse memperbesar atau memperkecil di sekitar penunjuk. Seret dengan tombol kanan atau tengah untuk menggeser pandangan. Klik dua kali tombol kanan untuk membingkai permukaan lagi; memilih permukaan lain otomatis membingkainya.
- Seret dengan tombol kiri untuk memindahkan material sementara garis luar permukaan tetap di tempatnya. Cincin memutar pemetaan, handle sudut menskalakannya, dan handle tengah menentukan titik asal perubahan tersebut.
- Seret dengan tombol kiri sambil menahan `Alt` pada tepi ubin untuk menggeser miring, begitu pula seret tombol tengah sambil menahan `Shift`.
- Setiap seretan adalah satu langkah urungkan, dan `Esc` membatalkan seretan yang sedang berlangsung.

Di bawah kanvas ada **Pergeseran**, **Skala**, **Sudut**, dan, jika game menggunakannya, **Flag konten**, **Flag permukaan**, serta **Nilai permukaan**. Tombolnya adalah **Atur ulang**, **Atur ulang ke dunia**, **Sesuaikan**, **Balik H**, **Balik V**, dan dua tombol putar 90°. **Sejajarkan** memutar material searah salah satu tepi permukaan (tekan lagi untuk tepi berikutnya, `Shift` untuk tepi sebelumnya). Empat tombol perataan mendorong material ke salah satu tepi. **Sesuaikan U** dan **Sesuaikan V** menskalakan agar sejumlah salinan utuh menutupi permukaan; tekan lagi untuk jumlah berikutnya, dan tahan `Ctrl` untuk mempertahankan subdivisi trim sheet.

## Tombol UV
Jika permukaan dipilih, tombol panah memindahkan material, `[` dan `]` memutarnya 15°, `,` dan `.` membagi dua atau menggandakan skalanya, `Ctrl+F` dan `Ctrl+Alt+F` membaliknya, `Shift+R` mengatur ulang, `Alt+Shift+R` mengatur ulang ke sumbu dunia, dan `Ctrl+Shift+F` menyesuaikannya dengan setiap permukaan.

**Kunci** di bilah alat mempertahankan pemetaan saat brush dipindahkan, dan **UV** (`U`) mempertahankannya saat brush diputar.

## Material alat
- skip: menghapus permukaan yang ditandai dari hasil ekspor ke mesin game.
- clip: membuat tabrakan tanpa geometri yang terlihat.
- origin: menandai titik poros sebuah entitas brush.

Nama-namanya dapat diubah di kartu **Material alat** pada jendela Ekspor. Gunakan skip untuk permukaan yang Anda tahu tersembunyi.

## Mengabaikan permukaan saat ekspor
Pilih sebuah permukaan dan aktifkan **Abaikan sisi saat ekspor** di bagian UV. Arsiran oranye menandainya di pratinjau UV. Material dan catnya tetap tersimpan, dan brush tetap tertutup serta dapat diedit. Mesh render hasil ekspor tidak menyertakan permukaan itu, tetapi tabrakannya tetap dipertahankan. Menghilangkan satu quad dari sebuah kubus menghasilkan dua segitiga render. Fitur ini tidak membuat permukaan menjadi dua sisi dan tidak menggabungkan mesh.

**Abaikan sisi yang tertutup** menandai permukaan yang sepenuhnya tertutup oleh brush statis, termasuk permukaan identik yang bersentuhan dan brush yang terkurung. Perintah ini bekerja pada brush yang dipilih, atau pada semua geometri dunia yang terlihat dan tidak terkunci jika tidak ada yang dipilih. Entitas brush, material transparan, dan brush yang sudah memiliki permukaan yang diabaikan akan dilewati. Permukaan yang hanya tertutup sebagian tidak dipecah. Satu kali urungkan memulihkan semuanya. Tampilan → Tampilkan sisi yang diabaikan menyembunyikan atau menampilkan permukaan semacam itu di tampilan editor; pemilihan tetap menggunakan brush secara utuh.

# Warna dan gradien
Bagian **Warna** di tab Permukaan mengecat permukaan dengan warna polos atau gradien di atas materialnya. Ini berguna untuk blockout level dan untuk gaya seni yang distilisasi. Permukaan yang dicat tetap menampilkan warna aslinya di tampilan; pilihan ditunjukkan oleh garis luarnya.

## Palet dan warna campuran
Klik salah satu dari dua belas contoh warna di palet untuk mengecat permukaan yang dipilih, atau seluruh brush yang dipilih, dengan warna itu. **Opasitas** menentukan seberapa pekat cat berikutnya, dalam persen. **Campuran** memiliki tombol warna yang membuka pemilih warna; seret di dalam pemilih dan pilihan langsung mengikuti, atau tekan **Cat** untuk menerapkan warna campuran sebagai satu langkah. **Kembali ke material** menghapus cat sehingga permukaan kembali menampilkan materialnya. Baris di bagian atas menyebutkan apa yang dipilih dan apa yang dikenakannya.

## Pemilih warna
Setiap tombol warna di editor membuka pemilih yang sama. Seret di dalam kotak untuk memilih saturasi dan kecerahan, dan di strip di sampingnya untuk memilih rona. Di bawahnya ada nilai **R**, **G**, dan **B** serta penggeser **Opasitas** jika warna tersebut memilikinya. Kolom heksadesimal menerima `#RRGGBB` atau `#RRGGBBAA`; kolom ini berubah merah selama teksnya bukan warna yang valid. **Salin** menaruh kode heksadesimal di papan klip. Klik di luar pemilih untuk menutupnya.

## Gradien
**Gradien** memilih Gradien linear atau Gradien radial, dan tombol di sampingnya menukar urutan warna dari ujung ke ujung. Setiap titik warna memiliki tombol warna dan **Posisi (%)**, dan titik di antara kedua ujung memiliki tombol hapus. **Tambah titik warna** menambahkan titik di celah terlebar; sebuah gradien dapat memiliki hingga delapan titik. **Sudut gradien** menentukan arah gradien linear, dengan tombol **+45°** dan **−45°**.

Di bawah pratinjau ada rel dengan penanda untuk setiap titik. Klik rel untuk menambah titik, klik penanda untuk membuka pemilihnya, seret penanda untuk memindahkannya, dan klik kanan penanda untuk menghapusnya. Tiga contoh warna di bawah rel adalah prasetel: Gradien hangat, Gradien sejuk, dan Gradien abu-abu. Perubahan pada gradien langsung diterapkan ke pilihan.

Arah gradien itu penting: arahkan sepanjang kemiringan atap atau sepanjang objek melengkung, bukan melintasi permukaan yang tidak berkaitan. Periksa model dari kedua sisi setelah mengecat.

## Material permukaan
**Material permukaan** mengatur **Kekasaran** (0 berarti mengilap, 100 berarti doff), **Metalik** (0 untuk kayu, batu, atau plastik, 100 untuk logam), dan **Emisi** (0 hingga 8; permukaan berpendar dengan warnanya sendiri, termasuk gradien, tetapi tidak menerangi objek di sekitarnya). Perubahan langsung diterapkan pada permukaan atau brush yang dipilih. **Gunakan permukaan bawaan** menghapus pengaturan khusus ini. Semua perubahan dapat diurungkan.

Efek permukaan terlihat di tampilan 3D yang diterangi dan di render kamera. Tampilan datar dan kerangka kawat sengaja tidak menampilkan kilau. Bloom dapat menambahkan pendaran di sekitar permukaan emisif. Godot dan glTF/GLB menerima nilai material, dan importer Unity mengubah kekasaran menjadi kehalusan (smoothness). Lihat `docs/surface-materials.md` untuk batasan setiap target.

# Aset, lapisan, dan grup
## Pustaka aset
Buka tab **Aset**. Tambahkan folder berisi model `.map` dengan **Tambah folder**; pustaka bawaan selalu tersedia. Ketik di **Filter aset** untuk mencari. Klik sebuah model untuk memilihnya, lalu klik dua kali, tekan `Enter`, atau klik **Sisipkan** untuk menambahkannya di posisi terakhir penunjuk. Anda juga dapat menyeret model ke sebuah tampilan. Brush aset disisipkan sebagai satu grup sehingga model bergerak sebagai satu kesatuan.

**Simpan pilihan sebagai aset** menulis brush yang dipilih ke pustaka sebagai `.map`. Aset pustaka adalah brush biasa yang dapat diedit, bukan mesh impor.

## Lapisan
Daftar **Lapisan** di tab Peta membantu mengatur area yang luas. Ketik nama lalu klik **Tambah** untuk membuat lapisan, atau tekan `Enter` untuk mengganti nama lapisan saat ini. Setiap lapisan dapat disembunyikan, dikunci, atau dikecualikan dari ekspor. **Pindahkan ke sini** memindahkan pilihan ke lapisan saat ini. Menu konteks di tampilan juga menyediakan Pindahkan ke lapisan, Jadikan lapisan aktif, Sembunyikan lapisan, dan Isolasi lapisan.

## Grup dan grup tertaut
`Ctrl+G` mengelompokkan pilihan dan `Ctrl+Shift+G` membubarkannya. Bagian **Grup** di tab Peta menampilkan daftar grup dan menyediakan **Buka**, **Ganti nama**, **Gabungkan**, **Bubarkan grup**, **Keluarkan**, dan **Ekstrak**. Grup tertaut adalah sekumpulan salinan yang berbagi suntingan: `Ctrl+Shift+D` membuat duplikat tertaut, Geometri → Sinkronkan grup tertaut memperbarui salinan-salinannya, dan **Putuskan tautan** membuat sebuah salinan menjadi mandiri. Di tab Entitas, sebuah properti dapat dipertahankan sebagai milik salinan itu sendiri saat grup tertaut disinkronkan.

## Skala bersama
Skala ekspor bawaan adalah 32 unit peta per meter. Penanda titik awal pemain bawaan setinggi 64 unit, yaitu acuan 2 meter. Bandingkan ambang pintu, kendaraan, perabot, dan tanaman dengannya. Ubah skala ekspor hanya jika memang disengaja dan jaga agar aset yang saling terkait tetap konsisten.

# Entitas dan gameplay
Definisi entitas yang umum sudah tersedia bawaan dan selalu dapat dipakai: lampu, titik awal pemain, pintu, tombol, platform, pemicu, prop statis, suara, dan waypoint. Memuat FGD proyek dengan **Definisi entitas** di tab Entitas menggantikan kelas yang bernama sama dan mempertahankan kelas bawaan lainnya.

## Menempatkan entitas
Di peramban **Entitas** pada tab Entitas, ketik di **Filter kelas** dan pilih sebuah kelas. Untuk kelas titik seperti lampu, klik **Tempatkan** atau klik dua kali kelasnya; `Insert` menempatkan satu entitas di tengah pilihan. Untuk kelas brush seperti pintu atau pemicu, pilih brush-nya lalu klik **Buat**. Kelas juga dapat diseret ke sebuah tampilan.

## Properti
Daftar properti mengedit entitas yang dipilih. Entitas dari kelas yang sama diedit bersamaan. **Tambah properti** menambahkan kunci yang mungkin tidak dikenal oleh definisinya; **Bawaan** dapat mengembalikan properti yang ada ke nilai bawaannya atau menambahkan properti yang belum ada. Properti yang didefinisikan kelas tetapi tidak dimiliki entitas ditampilkan redup; mengetikkan nilai akan menambahkannya. Gunakan targetname dan target untuk menautkan entitas.

## Apa yang bisa dilakukan ekspor
Direktif Godot membuat node native, badan tabrakan, dan area pemicu. Unity dan Unreal mendukung lampu umum secara native, dan Unreal juga membuat aktor PlayerStart. Data lainnya dipertahankan sejauh yang diizinkan importer target.

Entitas pintu saja tidak menambahkan perilaku game. Gerakan, respons pemicu, pengendali pemain, dan skrip proyek tetap perlu dikerjakan di mesin game. Defold mengekspor properti entitas dan objek tabrakan; Flax menempatkan model dan entitas generik. Periksa hasilnya di mesin game Anda sebelum membangun gameplay di atasnya.

# Animasi
Heron Brush dapat menampilkan pratinjau gerakan sederhana dan mengedit animasi keyframe entitas brush, lalu mengekspornya ke beberapa mesin game.

## Pratinjau animasi
Aktifkan **Pratinjau animasi** di menu Tampilan, menu Animasi, menu Pengaturan kisi, atau bagian Tampilan di tab Peta. Fitur ini memutar rotasi func_rotate dan gerakan bolak-balik func_shuttle saat start_on diaktifkan, menggunakan properti sumbu, kecepatan, arah, jarak tempuh, dan waktu tunggu milik entitas, serta memutar klip keyframe yang disetel ke Putar saat mulai. Aset kipas dan kincir angin sudah berisi bagian yang berputar.

Mematikan pratinjau mengembalikan pose semula. Klik di sebuah tampilan menghentikan pemutaran sebelum memilih atau mengedit; navigasi kamera tetap berfungsi. Pratinjau tidak mengubah peta, riwayat urungkan, maupun ekspor, dan tidak menjalankan fisika atau logika game.

## Editor keyframe
Buka melalui Animasi → Animasi keyframe atau dari menu Pengaturan kisi. Editor ini menempel di bawah tampilan, atau di jendela 3D selama jendela itu terpisah. Ikon jendela di judulnya memindahkannya ke jendela tersendiri; frame dan riwayat urungkan tetap dipertahankan. **Edit animasi** mengalihkan tampilan ke pose animasi dan **Edit peta** kembali ke geometri diam. Dalam mode animasi, setiap tampilan menampilkan **Edit animasi** dan nomor frame di bagian bawah.

Pilih entitas brush, atau brush atau grup brush, untuk menganimasikannya. Grup brush datar berbagi satu animasi dan satu titik poros; grup yang berisi beberapa entitas memerlukan animasi terpisah.

## Klip
Pemilih **Animasi** beralih di antara klip-klip bernama. Ketik nama di **Nama animasi** lalu klik **Simpan sebagai** untuk menyalin klip saat ini dengan nama tersebut; **Hapus** menghapus klip. **Geser** membuat klip `translate` tanpa rotasi. **Animasi pintu** membuat klip `open` dan `close` yang mulus dengan putaran 90 derajat di sekitar titik poros saat ini; pindahkan dulu titik poros ke engsel. **Putar saat mulai** membuat klip diputar saat level dimulai; prasetel pintu menonaktifkannya.

## Kontrol transport dan pemutaran
Tombol transportnya adalah **Awal**, **Kunci sebelumnya**, **Pemutaran**, **Kunci berikutnya**, dan **Akhir**. **Bingkai** menampilkan dan mengatur frame saat ini, dan **Durasi (d)** mengatur panjang klip. Modenya adalah **Sekali**, **Berulang**, atau **Bolak-balik**.

## Linimasa
Linimasa memiliki penggaris dengan kursor putar dan jalur untuk **Posisi** (X, Y, dan Z diberi kunci bersamaan) serta untuk setiap sudut rotasi, RX, RY, dan RZ. Klik atau seret di penggaris untuk memindahkan kursor putar dan menampilkan pratinjau pose itu di setiap tampilan. Seret dengan tombol tengah untuk menggeser linimasa. Seret kunci untuk memindahkannya dalam waktu; `Shift` menambahkan ke pilihan; klik dua kali sebuah jalur untuk menyisipkan kunci; `Ctrl`+roda mouse untuk zoom. **Kurva** menampilkan kurva nilai sebagai pengganti jalur.

Tombol di atasnya adalah **Tambah kunci**, **Buang**, **Salin**, **Tempel**, dan **Duplikasi**. **Tempel ke bingkai** menjaga kunci tetap pada frame bulat dan **Sesuaikan** menampilkan seluruh klip. **Ubah waktu keyframe** membuka **Skala waktu**, **Pergeseran (frame)**, **Skalakan dari kursor putar**, **Perpanjang durasi**, dan **Ubah waktu pilihan**.

Inspektur kunci di samping linimasa mengedit kunci yang dipilih: **Bingkai**, **Nilai** (atau tiga koordinat untuk kunci Posisi), interpolasi **Linear**, **Tetap**, atau **Bézier**, **Kemiringan masuk** dan **Kemiringan keluar**, serta **Haluskan garis singgung**.

## Pintasan di editor
Selama linimasa memegang fokus keyboard: `Space` memutar atau menjeda, panah kiri dan kanan melangkah satu frame, `Ctrl+A` memilih semua kunci, `Ctrl+C`, `Ctrl+X`, dan `Ctrl+V` menyalin, memotong, dan menempel kunci, `Ctrl+D` menduplikasinya, `Delete` menghapusnya, `Ctrl+Z` mengurungkan, dan `Esc` membatalkan seretan. Di luar linimasa, `Esc` keluar dari mode animasi. Urungkan mempertahankan zoom, pilihan, dan kursor putar di tempatnya.

## Mengatur pose di tampilan
Baris di bawah linimasa berisi **Onion skinning** dengan jumlah dan **Jarak**, yang menampilkan pose di sekitarnya sebagai bayangan; koordinat **Titik poros**; **Keyframe otomatis**; serta alat pose **Geser**, **Putar**, dan **Titik poros**.

- **Tambah kunci** menyalakan Keyframe otomatis dan menjeda pemutaran. Dengan Keyframe otomatis menyala dan **Geser** dipilih, seret sumbu berwarna pada handle entitas yang dipilih di tampilan 3D untuk memberi kunci posisinya pada frame saat ini.
- Di tampilan 2D, seret brush yang dipilih atau anggota grupnya untuk memberi kunci translasi sepanjang dua sumbu yang terlihat, dengan pelekatan ke kisi. Keyframe otomatis tidak perlu menyala untuk ini.
- Dengan **Putar**, seret brush di tampilan 2D mengelilingi penanda titik poros oranye. Tampilan atas memberi kunci RZ, tampilan depan RY, dan tampilan samping RX, dilekatkan ke sudut kisi.
- Dengan **Titik poros**, seret pusat rotasi kuning di tampilan 2D mana pun. Titik ini melekat ke kisi dan mempertahankan kedalamannya. Kunci dan geometri diam tidak berubah.

Lintasan sian di 2D dan 3D menunjukkan pergerakan titik poros. Klik sebuah titik lintasan untuk memilih waktu dan kuncinya, atau seret untuk mengubah posisinya; dengan Keyframe otomatis menyala, Anda juga dapat menyeret titik lintasan di tampilan 3D. Label menunjukkan nomor frame dan segmen putus-putus menandai lompatan. `Esc` membatalkan seretan mana pun, dan satu kali urungkan membalikkan seluruh seretan.

Animasi baru dimulai dalam keadaan diam dengan kunci pada waktu nol. Buat gerakan dengan mengubah pose pada frame lain. Untuk lintasan berbentuk L yang tajam, beri kunci pada awal, sudut, dan akhir, lalu gunakan Linear dengan Bolak-balik.

## Menghubungkan tombol ke animasi
Pilih func_button, trigger_multiple, atau trigger_once. Tab Entitas menampilkan **Pemicu animasi** di atas properti. Pilih target dari daftar, atau klik **Pilih target dalam tampilan** lalu objek beranimasi di tampilan mana pun; `Esc` membatalkan pemilihan. Target atau klip yang hilang dilaporkan di bawah kontrol. Tautan entitas menampilkan panah ke targetnya saat tampilan tautan aktif.

**Tindakan** memilih apa yang terjadi saat aktivasi: **Putar** memulai klip bernama atau animasi yang dipilih; **Alihkan** bergantian antara `open` dan `close` jika keduanya ada, jika tidak memulai ulang animasi yang dipilih; **Buka** dan **Tutup** memulai klip tersebut; **Jeda** menahan pose, **Lanjutkan** meneruskan, dan **Hentikan** mengembalikan pose semula. **Hanya sekali** mengesampingkan bawaan kelas. **Jeda pengulangan (dtk)** menentukan waktu terpendek antaraktivasi; ini tidak menunda aktivasi pertama.

**Uji pemicu** menjalankan tindakan di pratinjau tanpa mengedit peta atau menambah langkah urungkan. **Atur ulang pratinjau** mengembalikan pose semula serta menghapus status sekali dan cooldown. Ini tidak menyimulasikan tabrakan pemain atau skrip game.

## Animasi dalam ekspor
Godot mengekspor resource AnimationPlayer dan AnimationLibrary native, termasuk kurva dan tabrakan yang bergerak. CharacterBody3D yang memasuki area tombol mengaktifkan targetnya. Suntingan pada klip ini di Godot tidak kembali ke peta dan akan ditimpa oleh ekspor berikutnya.

Ekspor raylib menambahkan `<map>_animation.h` dengan pemutar C dan `<map>_preview.c` dengan pratinjau kamera bebas; ikuti `<map>_raylib.md` untuk membangunnya. Di pratinjau, `SPACE` menjeda, `R` memulai ulang klip bawaan, dan `N` berganti di antara klip bernama. Defold menyematkan pemutaran di skrip entitas dengan pesan `heron_*_animation` yang terdokumentasi. Build Level di Unity menambahkan `HeronBrushAnimation` dan body kinematik. Ekspor Bevy menyertakan `heron_animation.rs` beserta petunjuknya. Godot, Unity, Defold, Bevy, dan raylib mempertahankan tautan pemicu, penanda sekali, dan jeda pengulangan. Game Anda tetap harus mendaftarkan pemutar dan menghubungkan tombolnya sendiri ke API masing-masing runtime.

## Contoh Motion lab
Buka `motion_lab` dari Contoh di layar sambutan. Contoh ini berisi platform berlintasan L, lift, platform berputar, dan pintu berengsel dengan tombol. Buka `godot/project.godot` di sampingnya dengan Godot lalu tekan `F5`. WASD dan mouse menggerakkan pemain, Space untuk melompat, 1 hingga 4 mengunjungi stasiun-stasiun, dan R memulai ulang stasiun saat ini. Injak bantalan berwarna untuk mengaktifkan targetnya. README yang disertakan menjelaskan cara mengekspor ulang.

# Mengekspor ke mesin game
Berkas → Ekspor membuka jendela Ekspor. Mesin-mesin game tercantum di sebelah kiri beserta logonya: Godot, Defold, three.js, Bevy, raylib, Unity, Unreal Engine, Flax Engine, Cocos Creator, Wavefront .obj, glTF .gltf, GLB .glb, dan terakhir `.map`. Halaman di sebelah kanan menjelaskan apa yang akan diterima target. **Tulis** menulisnya, **Periksa** membangun level tanpa menulisnya dan menampilkan daftar masalah, dan **Tutup** menutup jendela.

Ekspor terpisah dari penyimpanan: `.map` tetap menjadi sumber yang dapat Anda edit. Setiap kontrol langsung menyimpan pengaturan mesinnya begitu diubah, sehingga setiap mesin menyimpan pilihannya sendiri dan jendela terbuka seperti saat Anda meninggalkannya.

## Opsi umum
- **Skala**: **Unit per meter**, bawaannya 32.
- **Yang akan ditulis**: **Geometri tabrakan**.
- **Optimasi**: **Kelompokkan mesh statis dalam sel 1024 unit**, **Hapus permukaan yang tertanam di dalam geometri**, **Gabungkan permukaan sebidang**, dan **Gabungkan bentuk tabrakan**.
- **Jalur proyek**: lokasi level dan resource-nya di dalam proyek.
- **Material alat**: nama material clip, skip, dan origin.
- **Lainnya**: sakelar khusus mesin seperti **Beri nama simpul berdasarkan**, **Bangun kembali lapisan dan grup sebagai simpul induk**, **Balik urutan titik segitiga**, pembalikan koordinat tekstur, dan set UV kedua untuk cahaya bake dengan **Texel lightmap**.
- **Mengikuti perubahan peta**: **Tulis ulang setiap kali peta disimpan**. Opsi ini tersedia setelah mesin tersebut diekspor sekali, karena ekspor itulah yang menentukan lokasi penulisan.

Sel statis menetapkan brush dunia berdasarkan titik tengahnya dan menggabungkannya per material di dalam setiap sel 1024 unit, sehingga mesin game dapat melakukan culling pada bagian peta yang lebih kecil. Tabrakan tetap dipertahankan dan entitas gameplay tetap terpisah. Ini bisa menghasilkan lebih banyak draw call dibanding satu mesh dunia, sehingga nonaktif secara bawaan. Ekspor OBJ, glTF, dan GLB mandiri tidak menggunakan sel.

## Catatan per target
- Godot: jalur `res://` tekstur dan material, lapisan fisika pemicu, dan UV2 lightmap. Pilih folder ekspor di dalam proyek Godot.
- Defold: direktori resource aset, material model, dan sampler tekstur, yang harus ada di material yang dipilih.
- Unity: mesh OBJ dan importer editor; ekspor ke dalam folder Assets proyek.
- Unreal Engine: skrip impor yang dibuat otomatis dan jalur konten, biasanya /Game/<level>.
- Flax Engine: jalur konten model relatif terhadap folder Content proyek.
- three.js, Bevy, raylib, dan Cocos Creator: scene atau kode untuk runtime tersebut, dengan petunjuk di samping hasil ekspor.
- glTF / GLB: geometri portabel untuk alat yang kompatibel; GLB adalah bentuk binernya. OBJ: geometri dan material untuk alat pemodelan.
- `.map`: **Tulis sebagai** memilih Valve 220, Standar, Quake 2, atau Quake 2 (Valve). Jendela menyebutkan apa yang akan hilang pada dialek yang dipilih.

Baca peringatan setelah menulis, misalnya tekstur yang hilang. **Hasil pemeriksaan** menampilkan daftarnya di jendela.

# Tautan langsung: Godot, Unity, Defold, dan Flax
Tautan langsung mengirim suntingan yang belum disimpan dari Heron Brush ke editor mesin game selagi Anda bekerja. Arahnya hanya satu: suntingan di mesin game tidak kembali. Satu tautan langsung dapat berjalan per jendela peta. Hentikan tautan sebelum berganti mesin, dan mulai ulang untuk menerapkan opsi ekspor yang diubah. Kontrolnya ada di jendela Ekspor, pada halaman mesin tersebut, di kartu **Tautan langsung dengan**.

Indikator di bilah judul menampilkan status tautan; klik untuk menghentikannya. Menutup atau mengganti peta akan menghentikan publikasi. Setelah impor gagal, perbaiki masalahnya lalu mulai tautan lagi.

## Godot
1. Ekspor sekali ke dalam proyek Godot.
2. Klik **Pasang addon Godot**.
3. Aktifkan Heron Brush Live di Project Settings → Plugins milik Godot.
4. Klik **Mulai tautan langsung** lalu lampirkan peta dari dock Godot ke scene 3D yang terbuka.

Addon hanya mengganti cabang yang dibuatnya sendiri dan mempertahankan penempatan lampiran serta node lainnya. Simpan scene Godot sendiri, dan letakkan skrip serta node Anda di luar cabang yang dibuat otomatis.

## Unity
1. Ekspor sekali ke dalam folder Assets proyek Unity.
2. Klik **Pasang addon Unity** dan biarkan Unity mengompilasinya.
3. Buka Window → Heron Brush → Live Link di Unity.
4. Mulai tautan dan lampirkan peta ke scene Unity yang aktif.

Biarkan jendela live Unity tetap terbuka. Pembaruan dijeda dalam Play Mode dan Prefab Mode. Hanya child Generated dari objek peta yang diganti. Jangan edit berkas yang dibuat otomatis.

## Defold
1. Ekspor sekali ke dalam proyek Defold Anda dan biarkan proyek tetap terbuka di Defold.
2. Klik **Mulai tautan langsung**. Tidak perlu addon.
3. Buka heron_brush_live/preview.collection di Defold, atau rujuk dari scene Anda sendiri.

Tautan menulis ke folder terpisah yang dibuat otomatis dan tidak menyentuh ekspor akhir maupun game.project. **Muat ulang game yang berjalan** juga memperbarui game yang dijalankan dari Defold; menambahkan objek atau mengubah tabrakan mungkin mengharuskan level dimulai ulang. Koneksi yang gagal dicoba ulang secara otomatis.

## Flax
Ekspor ke dalam folder Content proyek dan klik **Pasang addon Flax**. Biarkan Flax mengompilasinya, mulai tautan, lalu pilih Tools → Heron Brush: attach live map di scene Flax yang terbuka. Root yang dibuat otomatis mempertahankan penempatannya dan objek lain tidak disentuh. Simpan scene sendiri. Pembaruan dijeda dalam Play Mode. Tools → Heron Brush: stop live map menghentikan sementara penerimaan.

# Sesi langsung kolaboratif
Kolaborasi → Sesi langsung membuka jendela untuk menjadi host atau bergabung dengan pengguna Heron Brush lain. Fitur ini berbagi pengeditan peta antarorang; terpisah dari tautan langsung ke mesin game.

## Menjadi host dan bergabung
Isi **Nama Anda**. Untuk menjadi host, pilih **Host sesi**, klik **Mulai sesi**, lalu kirim detail dari **Salin detail koneksi** ke tamu Anda. Untuk bergabung, pilih **Gabung sesi**, tempel detailnya, lalu klik **Gabung**. Bergabung akan mengganti peta yang terbuka dengan peta milik host, jadi simpan pekerjaan Anda terlebih dahulu.

Build standar bekerja di jaringan lokal, dan **Port jaringan lokal** menentukan port-nya. Sesi melalui internet (**Hubungkan melalui internet**) memerlukan build dengan fitur jaringan opsional. Peserta dapat melihat penanda kamera dan nama satu sama lain di tampilan 3D.

Menutup jendela tidak menghentikan sesi; indikator di bilah judul menampilkan statusnya. **Hentikan sesi** atau **Tinggalkan sesi** mengakhirinya. Jika dua orang mengedit objek yang sama, urutan di sisi host menentukan versi akhirnya, dan urungkan juga merupakan suntingan bersama. Sebisa mungkin kerjakan area yang berbeda, dan simpan titik pemeriksaan.

# Tampilan 3D dan monitor kedua
## Mode tampilan
Bagian **Tampilan** di tab Peta mengatur apa yang digambar di tampilan: Brush, Entitas titik, Model entitas, Nama kelas entitas, Garis luar grup, Patch, Material, Tampilkan sisi yang diabaikan, Pratinjau animasi, Kerangka kawat, Tepi, dan Garis luar entitas brush. Kerangka kawat menggambar tepi segitiga, termasuk diagonal dan tepi di balik permukaan, tanpa material. Ini adalah tampilan debug sementara dan tidak memengaruhi ekspor maupun render.

Pencahayaan, bayangan, oklusi ambien, pantulan, bloom, dan penghalusan untuk tampilan 3D ada di Preferensi → Perender. Ini adalah pengaturan pratinjau; gambar akhir menggunakan pengaturan kamera render itu sendiri.

## Jendela 3D terpisah
Tampilan → Tampilan 3D di jendela terpisah memindahkan tampilan 3D ke jendelanya sendiri, yang dapat Anda letakkan di monitor lain. Jendela ini berbagi kamera, dokumen, dan alat, sementara tampilan datar mengisi ruang di jendela utama. Menunya menyediakan Material, Tampilan solid, dan Kerangka kawat; submenu Tampilan berisi Tampilkan sisi yang diabaikan, Pratinjau animasi, Tepi, Garis luar grup, Entitas titik, Model entitas, dan Garis luar entitas brush; Fokus pada pilihan; Depan, Samping, dan Atas, yang memutar kamera mengelilingi pilihan (atau titik di depan kamera); Tempatkan kamera dari tampilan 3D; Layar penuh (`F11`); Render gambar; dan Kembali ke jendela utama. Menutup jendela itu menempelkan tampilan kembali ke jendela utama.

## Langit prosedural
Langit prosedural aktif secara bawaan di tampilan 3D dan dapat dimatikan di Preferensi → Perender → Cahaya. Kamera render memiliki sakelar **Langit prosedural** sendiri, yang disimpan bersama peta. Warna langit, cakrawala, dan tanah berasal dari pengaturan lingkungan, dan matahari mengikuti pencahayaan scene. Langit tidak memiliki geometri dan tidak diekspor.

# Kamera render dan gambar PNG
Berkas → Render gambar membuka jendela render: kamera dan pengaturannya di sebelah kiri dan pratinjau langsung di sebelah kanan yang digambar ulang saat Anda mengubahnya. Kamera disimpan di `.map` dalam lapisan Render cameras khusus editor yang dikecualikan dari ekspor ke mesin game.

## Menempatkan kamera
Bingkai bidikan di tampilan 3D lalu tekan `Ctrl+Alt+0` (atau `Ctrl+Alt+Num0`), atau klik **Tempatkan kamera dari tampilan 3D**. Jika tidak ada kamera yang dipilih, kamera tersimpan pertama yang diperbarui; jika belum ada, kamera baru dibuat. Langkah ini dapat diurungkan dan mempertahankan pengaturan keluaran serta pencahayaan. **Pilih kamera dalam peta**, **Muat kamera dari peta**, dan **Pindahkan tampilan 3D ke kamera** bekerja ke arah sebaliknya. **Bingkai kamera di tampilan 3D** menampilkan batas keluaran, dan **Kendalikan kamera dari tampilan 3D** memungkinkan navigasi biasa menggerakkan kamera yang tersimpan.

## Pengaturan
- **Posisi kamera**: X, Y, Z, yaw, dan pitch.
- **Keluaran dan lensa**: **Prasetel** (Full HD, Persegi, Potret), lebar, tinggi, sampel, **Bidang pandang vertikal**, dan **Batas pandang jauh**.
- **Pencahayaan**: Cahaya siang, Senja, atau Mendung, atau atur eksposur, matahari, warna langit, cakrawala, dan tanah, **Langit prosedural**, **Bayangan**, **Oklusi ambien**, dan **Bloom**.
- **Kedalaman bidang**: **Aktifkan kedalaman bidang**, jarak fokus, rentang tajam, dan radius blur. **Fokus pada pilihan** memfokuskan ke tengah objek terpilih yang ada di depan kamera.

**Simpan kamera dan pengaturan** menyimpan formulir ini di dalam peta; simpan petanya agar tersimpan di disk.

## Rendering
**Render PNG** menanyakan lokasi penyimpanan dan memulai render progresif, yang terisi sampel demi sampel di pratinjau. Lebih banyak sampel tampak lebih baik tetapi butuh waktu lebih lama. **Batalkan render** berhenti tanpa menulis berkas. Perender memakai pipeline raster editor, bukan path tracing. PNG memiliki latar belakang yang tidak transparan, dan animasi tidak dirender. Gambar yang sangat besar dapat membuat editor terhenti sesaat.

# Preferensi
Berkas → Preferensi (`Ctrl+,`) membuka pengaturan. Perubahan langsung diterapkan dan disimpan. Bilah samping menampilkan Ringkasan, Game, Tampilan, Warna, Mouse, Keyboard, dan Perender. Kolom pencarian di bagian atas menemukan pengaturan berdasarkan nama atau deskripsi di halaman mana pun. Setiap pengaturan yang diubah memiliki tombol sendiri untuk mengembalikan nilai bawaannya, dan **Pulihkan bawaan** mengatur ulang halaman saat ini.

## Halaman-halaman
- **Ringkasan**: pengaturan umum dalam satu tempat: Penampilan (Opasitas panel (%), Skema warna, Tata letak, Bahasa), Pengeditan, Tampilan, dan Panduan.
- **Game**: game, lokasi berkasnya, dan nama kolaborasi Anda. Sebuah game menentukan format peta yang ditulis, definisi entitas yang dimuat, dan lokasi materialnya. Konfigurasi game dibaca dari folder `games` di folder konfigurasi (`~/.config/heron-brush/games` di Linux dan macOS, `%LOCALAPPDATA%\heron-brush\games` di Windows) dan dari folder `games` di samping aplikasi, tempat paket menaruh konfigurasi bawaan.
- **Tampilan**: Tata letak, Skema warna, Bahasa, sumbu dunia, opasitas kisi, ukuran sel pemeriksaan kebocoran, dan kunci pemetaan.
- **Warna**: warna tampilan, geometri, dan handle.
- **Mouse**: kecepatan melihat, menggeser, dan bergerak beserta pembalikannya, kecepatan terbang, bidang pandang, Tautkan tampilan 2D, dan ukuran handle.
- **Keyboard**: pemetaan tombol, Heron Brush atau Blender.
- **Perender**: shading realistis, garis luar pilihan, cahaya, bayangan, oklusi ambien, pantulan, bloom, dan permukaan.

## Bahasa
**Bahasa** menyediakan Ikuti sistem dan setiap bahasa yang telah diterjemahkan. Antarmuka langsung berubah. Panduan dibuka dalam bahasa antarmuka jika terjemahannya tersedia, dan dalam bahasa Inggris jika tidak.

# Plugin dan MCP
## Plugin
Plugin memperluas editor dengan skrip Rhai. Berkas → Plugin membuka jendela Plugin. **Pasang plugin** memasang berkas `.rhai` dan **Pasang folder plugin** memasang folder yang berisi `main.rhai`; metadata `plugin.toml` opsional menjelaskan plugin dan pengaturannya. Plugin dikelompokkan di bawah **Plugin aktif** dan **Plugin nonaktif**, dan **Aktifkan plugin** menyalakan atau mematikannya; pengaturan ini tetap berlaku setelah aplikasi dimulai ulang. **Muat ulang plugin** membaca ulang skrip dari disk dan mengatur ulang nilai panelnya.

Plugin aktif juga muncul di bagian **Plugin** pada tab Peta beserta pengaturan, tombol tindakan (**Jalankan**), dan keluarannya. Sebuah tindakan bekerja pada pilihan, atau pada semuanya jika tidak ada yang dipilih, dan merupakan satu langkah urungkan. Selection inspector bawaan menampilkan jumlah objek serta tindakan untuk memindahkan ke atas dan ke bawah.

## MCP
Kolaborasi → Jalankan server untuk agen (MCP) memungkinkan agen AI atau alat lain mengendalikan editor yang terbuka melalui Model Context Protocol. Jendelanya menampilkan alamat lokal dengan **Salin**, dan **Hentikan server** mengakhirinya; selama server berjalan, bilah judul menampilkan **MCP aktif**. Opsi baris perintah `--mcp` menjalankan antarmuka yang sama tanpa jendela. Klien dapat menemukan alat untuk geometri, aset, properti, animasi, ekspor, dan sesi. MCP dan plugin editor adalah antarmuka yang berbeda.

# Baris perintah
Programnya adalah `heron-brush`. Tanpa opsi, program membuka layar sambutan.

## Membuka berkas
`heron-brush map.map [entities.fgd] [materials/]` membuka peta, secara opsional dengan definisi entitas dan folder material. Argumen kosong melewati satu argumen, misalnya `heron-brush map.map "" materials/`. Jika editor sudah berjalan, peta dibuka di dalamnya; `--new-instance` membuka jendela terpisah sebagai gantinya.

## Perintah
- `--help` atau `-h`: menampilkan cara penggunaan. `--version` atau `-V`: menampilkan versi.
- `--mcp`: menyajikan MCP melalui standard input dan output, tanpa jendela.
- `--merge-map <base> <ours> <theirs> [<output>]`: penggabungan tiga arah berkas `.map` untuk git. Keluarannya secara bawaan adalah ours. Konflik dicetak dan menghasilkan kode keluar bukan nol.
- `--obj-to-map <in.obj> <out.map> [size] [thickness] [snap] [material]`: mengubah mesh menjadi brush. Size adalah panjang sisi terpanjang dalam unit peta (bawaan 256), snap adalah kisi untuk sudut-sudutnya (0 berarti tanpa kisi), dan kata `solid` atau `layered` membangun kotak kisi atau lempengan cembung, bukan irisan tipis.
- `--render-preview <in.map> <out.png> [size]`: menulis gambar mini PNG dari sebuah peta, bawaannya 512 piksel.
- `--run-script <script.rhai> <in.map> [out.map] [key=value ...]`: menjalankan skrip plugin pada peta dengan semuanya terpilih lalu menulis hasilnya (secara bawaan kembali ke berkas masukan). `key=value` menjawab pengaturan plugin.

# Pemecahan masalah
## Jika ada yang tampak salah
- Material hilang: periksa folder atau paket material serta jalur proyek pada ekspor.
- Ukuran model salah: bandingkan dengan penanda pemain setinggi 64 unit dan skala ekspor.
- Arah gradien salah: periksa sudut gradien dan orientasi permukaan.
- Scene mesin game kosong: baca peringatan ekspor dan ikuti petunjuk impor untuk target tersebut.
- Tautan langsung terus menunggu: pastikan addon sudah terpasang, scene sudah dilampirkan, dan pembaruan tidak dijeda.
- Hanya garis, tanpa permukaan: matikan Kerangka kawat di bagian Tampilan pada tab Peta, atau pilih Material atau Tampilan solid di menu jendela 3D.
- Objek hilang dari tampilan: `Ctrl+Shift+I` menampilkan objek tersembunyi; periksa lapisan dan filter di bagian Tampilan.
- Tombol tidak berfungsi: klik dulu di tampilan agar tampilan itu memegang fokus keyboard, dan periksa pemetaan tombol di Preferensi → Keyboard.
- Suntingan gagal: urungkan, sederhanakan geometrinya, lalu gunakan **Periksa peta** dan **Periksa kebocoran** di bagian **Masalah** pada tab Peta.
- Peta tidak mau terbuka dari pengelola berkas: peta itu mungkin diserahkan ke editor yang sudah berjalan; cari jendela tersebut.

## Pengaturan dan cadangan
Preferensi, peta terbaru, bintang, pengaturan ekspor, dan plugin tersimpan di folder konfigurasi: `~/.config/heron-brush` di Linux dan macOS, serta `%LOCALAPPDATA%\heron-brush` di Windows. Cadangan simpan otomatis ada di folder `autosave` di samping setiap peta.

## Melaporkan bug
Sertakan versi editor (Bantuan → Tentang), sistem operasi Anda, peta kecil yang memperlihatkan masalahnya, dan langkah-langkah yang persis. Panel keluaran (`Ctrl+4`) sering kali menyebutkan apa yang salah. Simpan salinan sebelum bereksperimen.

## Menggunakan panduan ini
Pilih bab di daftar isi di sebelah kiri. `Ctrl+F` menampilkan daftar isi dan berpindah ke kolom pencarian; `Esc` mengosongkan pencarian, atau menutup panduan jika pencarian kosong. **Daftar isi** menyembunyikan atau menampilkan daftar bab dan **Ukuran teks** mengubah ukuran huruf; keduanya diingat. Daftar bahasa di bagian atas beralih di antara terjemahan yang tersedia.
