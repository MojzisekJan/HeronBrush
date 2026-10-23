# Bắt đầu
Heron Brush là trình chỉnh sửa màn chơi dựa trên brush. Bạn dựng màn chơi từ các brush lồi, tô màu và gán vật liệu cho các mặt của chúng, đặt thực thể, tạo hoạt ảnh cho cửa và bệ di chuyển, rồi xuất kết quả sang engine trò chơi. Trình chỉnh sửa còn có tài nguyên dựng sẵn, hình tham số, liên kết trực tiếp với trình chỉnh sửa của engine, phiên chỉnh sửa chung và kết xuất ảnh tĩnh. Hướng dẫn này đi kèm ứng dụng và dùng được khi không có mạng.

Trên macOS, `Cmd` dùng được ở mọi chỗ hướng dẫn này ghi `Ctrl`, và Option là Alt.

## Màn hình chào mừng
Khi Heron Brush khởi động mà không có bản đồ, màn hình chào mừng sẽ mở ra. Bên trái là **Tiếp tục** (bản đồ bạn mở gần nhất), **Bản đồ mới** và **Duyệt** để mở một tệp `.map` có sẵn, cùng với **Trò chơi**, **Hướng dẫn sử dụng** và **Tùy chọn**. Bên phải là danh sách bản đồ gần đây kèm ảnh thu nhỏ, tiếp theo là **Ví dụ**. Ngôi sao (**Luôn ở trên cùng**) ghim bản đồ lên đầu danh sách; thùng rác (**Chuyển vào thùng rác**) chuyển tệp vào thùng rác của hệ thống, nên có thể khôi phục lại. Các phím mũi tên di chuyển trong màn hình và Enter mở mục đang được chọn. Tệp → Chào mừng đưa màn hình này trở lại bất cứ lúc nào.

## Bản đồ đầu tiên của bạn
1. Chọn **Bản đồ mới** trên màn hình chào mừng, hoặc Tệp → Bản đồ mới (`Ctrl+N`).
2. Chọn công cụ Brush (`B`) trên dải công cụ và kéo trong một khung nhìn 2D để vẽ hình đã chọn. Kiểm tra chiều sâu của nó trong một khung nhìn 2D khác.
3. Chọn brush và kéo để di chuyển nó. Dùng thẻ **Mặt** để gán cho nó một vật liệu hoặc một màu.
4. Lưu bản đồ nguồn bằng `Ctrl+S`. Khi sẵn sàng dùng nó trong trò chơi, mở Tệp → Xuất và ghi một cảnh cho engine của bạn.

## Tệp và lưu
Tệp `.map` là bản nguồn có thể chỉnh sửa của bạn. Hãy lưu thường xuyên: xuất, liên kết trực tiếp và kết xuất không bao giờ tự lưu bản đồ thay bạn. Tệp → Lưu thành (`Ctrl+Shift+S`) tự thêm phần mở rộng `.map` nếu bạn bỏ qua. Tệp → Hoàn nguyên về bản đồ đã lưu bỏ các thay đổi chưa lưu và tải lại tệp. Nếu bạn đóng cửa sổ hoặc mở bản đồ khác khi còn thay đổi chưa lưu, Heron Brush sẽ hỏi bạn muốn **Lưu**, **Bỏ** hay **Hủy**.

Khi một bản đồ đã lưu có thay đổi chưa lưu, các bản sao lưu đánh số được ghi mỗi mười phút vào thư mục `autosave` cạnh bản đồ. 50 bản sao lưu mới nhất được giữ lại.

Mở một bản đồ khi Heron Brush đang chạy sẽ chuyển nó cho cửa sổ đang chạy, cửa sổ đó sẽ được đưa lên trước. Dùng `--new-instance` trên dòng lệnh khi bạn muốn một cửa sổ riêng (xem Dòng lệnh).

# Giao diện
Cửa sổ chính có thanh tiêu đề chứa các menu, dải công cụ bên dưới, các khung nhìn bản đồ, bảng thuộc tính ở bên phải và thanh trạng thái dọc theo cạnh dưới.

## Thanh tiêu đề và menu
Thanh tiêu đề hiển thị logo, các menu, tên bản đồ ở giữa và các nút cửa sổ. Trên macOS, các nút đỏ, vàng và xanh quen thuộc nằm ở bên trái. Kéo thanh tiêu đề để di chuyển cửa sổ và nhấp đúp để phóng to. Khi có tác vụ chạy nền, một chỉ báo xuất hiện gần cuối thanh: liên kết trực tiếp với engine, phiên trực tiếp hoặc **MCP đang hoạt động**. Nhấp vào chỉ báo để dừng liên kết trực tiếp hoặc mở cửa sổ của nó.

- **Tệp**: Chào mừng, Bản đồ mới (`Ctrl+N`), Mở bản đồ (`Ctrl+O`), Lưu (`Ctrl+S`), Lưu thành (`Ctrl+Shift+S`), Hoàn nguyên về bản đồ đã lưu, Xuất, Kết xuất ảnh, Plugin và Tùy chọn (`Ctrl+,`).
- **Chỉnh sửa**: Hoàn tác (`Ctrl+Z`), Làm lại (`Ctrl+Shift+Z`), Cắt, Sao chép, Dán, Nhân bản (`Ctrl+D`), Xóa, Chọn tất cả (`Ctrl+A`), Lặp lại các lệnh gần nhất (`Ctrl+R`) và Quên các lệnh có thể lặp lại.
- **Hiển thị**: Khung nhìn 3D trong cửa sổ riêng, Đi đến, Bám theo lưới (`Alt+0`), Hiện lưới (`0`), Xem trước hoạt ảnh và Hiện các mặt bị loại.
- **Hình học**: Chèn chữ 3D, Nối góc, Chuyển mặt thành patch, Đưa vùng chọn ra khỏi thực thể của nó và Đồng bộ nhóm liên kết.
- **Hoạt ảnh**: Hoạt ảnh khung khóa, Xem trước hoạt ảnh và Đặt camera từ khung nhìn 3D (`Ctrl+Alt+0`).
- **Bản đồ**: Tải, Tải lại và Gỡ tệp điểm; Tải, Tải lại và Gỡ tệp portal. Chúng hiển thị vết rò rỉ và portal do trình biên dịch bản đồ ghi ra.
- **Cộng tác**: Phiên trực tiếp và Phục vụ cho agent (MCP).
- **Trợ giúp**: Hướng dẫn sử dụng và Giới thiệu.

## Dải công cụ
Dải công cụ chứa mọi công cụ. Nhóm đầu tiên được dùng thường xuyên: Chọn, Brush, Xén, Đỉnh, Cạnh, Mặt, Xoay và Tỷ lệ. Nhóm thứ hai gồm Xô nghiêng, Lắp ghép, Quét, Điểm điều khiển, Bo tròn và Sơn. Rê chuột lên một nút để xem tên và phím tắt. Nhấp lại vào công cụ đang dùng sẽ quay về Chọn.

Sau các công cụ là **Lật theo chiều ngang** và **Lật theo chiều dọc**, rồi đến hai công tắc: **Khóa** giữ nguyên ánh xạ vật liệu khi brush di chuyển, và **UV** giữ nguyên nó khi brush xoay (`U` bật/tắt nó). Tiếp theo là cài đặt riêng của công cụ đang dùng: công cụ Brush hiển thị hình và các tùy chọn của nó, còn công cụ Bo tròn hiển thị bán kính, số mặt và góc cùng nút **Áp dụng**. Ở đầu bên phải là tên công cụ, kích thước lưới (**Lưới 16** và tương tự; nhấp để chọn kích thước khác) và menu **Cài đặt lưới** với Bám theo lưới, Hiện lưới, Xem trước hoạt ảnh, Hiện các mặt bị loại và Hoạt ảnh khung khóa.

## Khung nhìn bản đồ
Có bốn khung nhìn: khung nhìn 3D và ba khung nhìn phẳng, Phía trên (XY), Phía trước (XZ) và Bên cạnh (YZ). Tùy chọn → Hiển thị → Bố cục chọn Một khung, Hai khung, Ba khung hoặc Bốn khung. Kéo các đường phân cách để đổi kích thước các khung. Nhấp vào một khung nhìn sẽ trao bàn phím cho nó. `Space` chuyển một khung hiển thị từng khung nhìn một sang khung nhìn tiếp theo, còn `Ctrl+Space` phóng to khung nhìn đang nhận bàn phím và khôi phục lại bố cục.

Nhấp chuột phải không kéo trong bất kỳ khung nhìn nào để mở menu ngữ cảnh cho vùng chọn và thứ nằm dưới con trỏ. Menu này có gom nhóm, lớp, nhóm liên kết, tạo thực thể, đưa brush vào hoặc ra khỏi thực thể, sao chép tên vật liệu và hiện vật liệu đó trong trình duyệt vật liệu.

## Bảng thuộc tính
Bảng thuộc tính bên phải có bốn thẻ. Kéo cạnh trái của nó để đổi độ rộng.

- **Bản đồ** (`Ctrl+1`): Lớp, Nhóm, Plugin, Vấn đề, bộ lọc hiển thị và Thẻ.
- **Thực thể** (`Ctrl+2`): trình duyệt thực thể, định nghĩa thực thể, thuộc tính của thực thể đang chọn và, với nút bấm và trigger, Kích hoạt hoạt ảnh.
- **Mặt** (`Ctrl+3`): UV, Vật liệu, Màu, Quét, Bo tròn, Patch, và Thu vào & vát. Mỗi phần đều có thể thu gọn.
- **Tài nguyên**: thư viện các mô hình brush dùng lại được.

`Ctrl+5` ẩn hoặc hiện bảng thuộc tính và `Ctrl+Alt+T` ẩn hoặc hiện dải công cụ, giúp các khung nhìn có thêm chỗ.

## Thanh trạng thái và khung đầu ra
Thanh trạng thái dọc cạnh dưới hiển thị thông báo mới nhất của trình chỉnh sửa. Lỗi được hiển thị màu đỏ. Nhấp vào thanh, hoặc nhấn `Ctrl+4`, để mở khung đầu ra với toàn bộ lịch sử thông báo. Phần lớn lệnh báo cáo việc chúng đã làm ở đây, ví dụ đã xén bao nhiêu brush.

## Hộp thoại và giao diện
Các hộp thoại như Tùy chọn, Xuất và Kết xuất ảnh mở thành cửa sổ riêng. Tùy chọn → Hiển thị → Bảng màu giao diện chọn Theo hệ thống, Sáng hoặc Tối. **Độ đục bảng (%)** trên trang Tổng quan làm các bảng trong suốt phía trên khung nhìn. Cửa sổ tự vẽ các nút của mình theo phong cách hệ thống: kiểu gốc trên macOS, kiểu Windows 11 trên Windows và kiểu GNOME trên Linux.

# Điều hướng và bàn phím
## Di chuyển trong khung nhìn 3D
Giữ nút chuột phải và kéo để nhìn quanh. Kéo bằng nút giữa để lia. Con lăn di chuyển tới và lui. `W`, `A`, `S` và `D` bay tới, sang trái, lùi và sang phải, `Q` bay lên và `X` hạ xuống; giữ `Shift` để bước dài hơn. Khi bật **Alt + kéo chuột giữa để di chuyển camera** trong Tùy chọn → Chuột, Alt cùng nút giữa di chuyển theo hướng nhìn. Tốc độ, đảo chiều và góc nhìn nằm trong Tùy chọn → Chuột.

## Di chuyển trong khung nhìn 2D
Kéo bằng nút giữa để lia và dùng con lăn để thu phóng quanh con trỏ. Lia và thu phóng một khung nhìn phẳng sẽ giữ các khung nhìn khác thẳng hàng; **Liên kết các khung nhìn 2D** trong Tùy chọn → Chuột điều khiển việc này. `Ctrl+Alt+Z` đặt lại mức thu phóng và `Ctrl+Shift+U` đưa các khung nhìn phẳng về gốc tọa độ.

`Ctrl+U` đưa vùng chọn vào khung hình trong mọi khung nhìn. Hiển thị → Đi đến (`Ctrl+Alt+M`) đưa camera đến tọa độ nhập vào, chọn thứ nằm ở một dòng của tệp bản đồ, hoặc di chuyển vùng chọn một khoảng nhập vào.

## Lưới và bám lưới
`+` và `-` thay đổi kích thước lưới, còn các phím số `1` đến `9` chọn trực tiếp một kích thước, từ 1 đến 256 đơn vị. `0` hoặc `#` hiện hoặc ẩn lưới, và `Alt+0` bật hoặc tắt bám lưới. `S` bám các đỉnh của brush đang chọn vào lưới hiện tại, `Ctrl+Shift+V` bám chúng vào đơn vị nguyên và `Ctrl+Alt+Shift+V` vào lưới.

## Phím tắt thông dụng
- `F3` hoặc `Ctrl+Shift+P`: tìm bất kỳ lệnh menu nào theo tên và chạy bằng `Enter`. `F10` mở các menu, và phím mũi tên di chuyển trong đó.
- `Ctrl+S`: lưu bản đồ. `Ctrl+Shift+S`: lưu thành.
- `Ctrl+Z`: hoàn tác. `Ctrl+Shift+Z` hoặc `Ctrl+Y`: làm lại.
- `Ctrl+C`, `Ctrl+X`, `Ctrl+V`: sao chép, cắt và dán. Dán đặt đối tượng tại con trỏ; `Ctrl+Alt+V` giữ nguyên tọa độ gốc của chúng. Khi đang chọn mặt, sao chép sẽ lấy vật liệu và cài đặt UV của chúng.
- `Ctrl+D`: nhân bản. `Ctrl+Shift+D`: bản sao liên kết của một nhóm.
- `Delete` hoặc `Backspace`: xóa vùng chọn.
- `Esc`: hủy thao tác kéo hiện tại, lùi một bước trong công cụ đang dùng, quay về Chọn, và cuối cùng bỏ vùng chọn. `Shift+Esc` quay thẳng về Chọn.
- `Ctrl+R`: lặp lại các lệnh có thể lặp lại gần nhất. `Ctrl+Shift+R`: quên chúng.
- `Ctrl+Alt+0` hoặc `Ctrl+Alt+Num0`: đặt camera kết xuất tại khung nhìn 3D.
- `F5`: tải lại vật liệu. `F6`: tải lại định nghĩa thực thể.

## Phím tắt chọn
- `Ctrl+A`: chọn tất cả, hoặc mọi tay nắm khi đang dùng công cụ đỉnh, cạnh hoặc mặt. `Ctrl+Shift+A`: bỏ chọn tất cả. `Ctrl+Alt+A`: đảo vùng chọn.
- `Ctrl+B`: chọn các đối tượng khác trong cùng thực thể hoặc nhóm.
- `Ctrl+T`: chọn mọi thứ mà các brush đang chọn chạm vào. `Ctrl+E`: chọn mọi thứ nằm bên trong chúng. `Ctrl+Shift+E`: chọn mọi thứ trong cột của chúng theo trục nhìn. Các lệnh này tiêu thụ các brush dùng để chọn.
- `Ctrl+I`: cô lập vùng chọn. `Ctrl+Alt+I`: ẩn nó. `Ctrl+Shift+I`: hiện lại mọi thứ.
- `Ctrl+G`: gom nhóm. `Ctrl+Shift+G`: rã nhóm.

## Phím tắt biến đổi
- Các phím mũi tên và `Page Up` / `Page Down` di chuyển vùng chọn một bước lưới theo hướng chúng chỉ trong khung nhìn đang dùng. Với `Ctrl` chúng nhân bản khi di chuyển, và với `Alt` chúng xoay.
- `Alt+X`, `Alt+Y`, `Alt+Z`: xoay một phần tư vòng quanh trục đó; thêm `Shift` để xoay chiều ngược lại.
- `Ctrl+F`: lật ngang theo khung nhìn; `Ctrl+Alt+F`: lật trên dưới. `Ctrl+Shift+X` và `Ctrl+Shift+Y` phản chiếu theo X hoặc Y.
- `Insert`: đặt một thực thể điểm tại tâm vùng chọn.
- `Tab`: khi đang chọn một brush, xoay vòng Chọn → Đỉnh → Cạnh → Mặt → Chọn.

## Sơ đồ phím Blender
Tùy chọn → Bàn phím → Sơ đồ phím có Heron Brush (mặc định, được dùng xuyên suốt hướng dẫn này) và Blender. Với Blender: `G` chọn và di chuyển, `R` xoay, `S` co giãn, `E` quét, `1`, `2` và `3` chọn công cụ đỉnh, cạnh và mặt, `K` xén, `B` chọn, `A` chọn mọi thứ và `Alt+A` bỏ vùng chọn, `X` xóa, `Shift+D` nhân bản, `H` ẩn và `Alt+H` hiện mọi thứ, còn `Ctrl+Shift+Z` làm lại. Mọi thứ khác giống nhau ở cả hai.

# Brush và công cụ dựng hình
Brush là khối đặc lồi được xác định bởi các mặt phẳng. Hãy dựng các vật thể phức tạp từ nhiều brush đơn giản thay vì cố làm một brush lõm. Giữ số lượng brush thấp ở những chỗ chỉ cần một hình đơn giản.

## Vẽ brush
Chọn công cụ Brush (`B`) và kéo trong một khung nhìn. Dải công cụ hiển thị **Hình để vẽ**: Hình hộp, Cầu thang, Vòm, Hình trụ, Hình nón, Khối cầu (UV), Khối cầu (Khối hai mươi mặt) và Chèn chữ 3D. `Alt+1` đến `Alt+7` chọn trực tiếp một hình. Tùy theo hình, dải công cụ hiển thị **Làm rỗng** và **Độ dày:**, **Chiều cao bậc:** và **Hướng:** cho cầu thang, **Trục:**, **Số cạnh bên:** và chế độ hình tròn cho các hình tròn, **Số vòng:** cho khối cầu và **Độ chính xác:** cho khối hai mươi mặt. Đặt các tùy chọn trước khi kéo. Kiểm tra kết quả trong nhiều khung nhìn.

## Chọn
Nhấp vào một brush để chọn nó, và nhấp kèm `Ctrl` để thêm vào vùng chọn. Kéo từ chỗ trống sẽ vẽ một hộp chọn. Trong khung nhìn 3D, nhấp kèm `Shift` chọn một mặt riêng lẻ và nhấp kèm `Ctrl+Shift` thêm nhiều mặt hơn. Kéo một brush đang chọn sẽ di chuyển nó; kéo một brush chưa chọn sẽ chọn và chỉ di chuyển brush đó.

Trong khung nhìn 3D, kéo kèm `Shift` một mặt của brush đang chọn để đùn nó ra. Nhấp kèm `Alt` vào một mặt khác để sao chép vật liệu và căn chỉnh của mặt đang chọn sang mặt đó; `Alt+Ctrl` chỉ sao chép vật liệu và `Alt+Shift` sao chép góc xoay.

## Công cụ tạo hình
- **Chọn** (`Esc`): chọn và di chuyển.
- **Đỉnh** (`V`), **Cạnh** (`E`), **Mặt** (`F`): kéo các góc, cạnh hoặc mặt. Kéo kèm `Shift` từ giữa một cạnh hoặc mặt sẽ thêm một góc ở đó. `Delete` xóa các góc đã chọn.
- **Xoay** (`R`), **Tỷ lệ** (`T`), **Xô nghiêng** (`G`): biến đổi vùng chọn bằng tay nắm.
- **Xén** (`C`): nhấp hai hoặc ba điểm để xác định mặt phẳng cắt. `Enter` cắt, `Ctrl+Enter` chọn phía cần giữ, và `Backspace` hoặc `Esc` xóa điểm cuối cùng.
- **Lắp ghép** (`N`): đặt các điểm quanh một khối và nhấn `Enter` để tạo một brush lồi từ chúng. `Backspace` xóa điểm cuối cùng.
- **Quét** (`Y`): xem phần Quét bên dưới.
- **Điểm điều khiển** (`P`): tạo lại hình cho lưới điểm điều khiển của patch.
- **Bo tròn** (`O`): nhấp vào một cạnh để bo tròn nó, hoặc chọn các cạnh hoặc đỉnh bằng công cụ của chúng rồi nhấn `Enter`. Nhiều mặt hơn cho kết quả mượt hơn và nhiều hình học hơn.
- **Sơn** (`K`): xem Vật liệu và UV.

## Kết hợp brush
- `Ctrl+J`: gộp các brush đang chọn thành bao lồi của chúng.
- `Ctrl+K`: trừ các brush đang chọn khỏi các brush mà chúng chạm vào.
- `Ctrl+Shift+K`: làm rỗng vùng chọn, với thành dày một bước lưới.
- `Ctrl+L`: lấy giao của vùng chọn.

## Quét, bo tròn, patch và vát
**Quét** tạo hình học dọc theo một đường từ các mặt đang chọn. Chọn các mặt (trong công cụ Quét, nhấp kèm `Alt`), rồi đặt **Hành trình**, **Xoay** (trục và góc), **Tỷ lệ**, **Số bậc** và **Đường dẫn** (căn chỉnh Số nguyên hoặc Tự do; Cung, Thẳng hoặc Uốn chữ S) trong phần Quét của thẻ Mặt, và nhấn **Quét**. Dòng bên dưới các điều khiển cho biết sẽ tạo bao nhiêu brush.

Phần **Bo tròn** bo tròn các cạnh đã chọn với **Bán kính**, **Mặt** và **Góc** (Cắt vát hoặc Bo tròn); một mặt sẽ tạo thành cạnh vát.

**Patch** hiển thị **Cột** và **Hàng** của patch đang chọn với các nút để thêm hoặc bớt một cặp. Hình học → Chuyển mặt thành patch (`Ctrl+P`) tạo patch từ các mặt đang chọn.

**Thu vào & vát** làm việc trên các mặt đang chọn: **Thu vào** kéo mặt vào trong từ các cạnh của nó và **Độ sâu** đẩy nó ra (hoặc vào, khi giá trị âm). **Vát mặt** áp dụng cả hai; nút thứ ba thêm một tấm ốp hoặc khoét một hốc, tùy theo dấu của Độ sâu. **Nối góc** mở hộp thoại nối hai brush tại một góc cắt vát hoặc bo tròn, có xem trước.

## Chữ 3D
Hình học → Chèn chữ 3D (cũng là mục cuối trong danh sách hình của công cụ Brush) tạo brush từ văn bản. Nó dùng Heron Block, một phông chữ đa giác dựng sẵn với góc vạt, độ rộng tỷ lệ và dấu tiếng Séc; chữ thường dùng dạng chữ hoa. Nhập **Chữ**, đặt **Chiều cao** (chiều cao chữ hoa; dấu nằm cao hơn) và **Độ sâu**, xem bản xem trước trực tiếp rồi nhấn **Chèn**.

Nếu một thao tác không thể tạo ra hình học hợp lệ, hãy điều chỉnh kích thước của nó, hoặc hoàn tác và đơn giản hóa hình.

## Mảng
Hình học → Mảng sao chép vùng chọn và hiển thị các bản sao trong bản đồ khi bạn thay đổi giá trị. Số lượng tính cả bản gốc, và một mảng tạo tối đa 1024 bản sao.

- **Hàng** dịch mỗi bản sao so với bản trước một khoảng **Bước X**, **Bước Y** và **Bước Z** và xoay nó **Xoay mỗi bản sao (độ)** quanh trục đã chọn.
- **Lưới** lấp đầy **Số lượng X**, **Số lượng Y** và **Số lượng Z** mục, cách nhau **Bước X**, **Bước Y** và **Bước Z**.
- **Vòng** rải các mục theo **Góc (độ)** quanh một tâm cách vùng chọn **Bán kính (đơn vị)**. Khi bật **Hướng về tâm**, mỗi bản sao xoay theo vòng.
- **Rải** đặt các bản sao ở vị trí ngẫu nhiên trong bán kính, ngang qua trục đã chọn. Cùng **Seed** cho cùng bố cục, và **Xoay ngẫu nhiên** xoay từng bản sao.

Khi vùng chọn là trọn một nhóm, **Bản sao liên kết** biến mỗi bản sao thành một bản nhân đôi liên kết, nên sửa một bản sẽ áp dụng cho tất cả. **Áp dụng** giữ các bản sao và chọn chúng cùng bản gốc; **Hủy** xóa chúng. Toàn bộ mảng được hoàn tác trong một bước.

## Điêu khắc địa hình
Địa hình gồm nhiều brush nhỏ mà cọ điêu khắc đẩy bề mặt ra và kéo vào. Chọn một hoặc nhiều brush rồi chọn Hình học → Chia thành cột. Lệnh này cắt chúng dọc theo các đường lưới thành các cột rộng một bước lưới và đưa các cột vào một nhóm tên Terrain. Các đường cắt theo lưới của thế giới, nên các brush nằm cạnh nhau bị cắt ở cùng chỗ và dùng chung góc. Mỗi cột đứng dọc theo cạnh mỏng của brush: đứng thẳng trong sàn hoặc trần, nằm ngang trong tường. Hang động cũng làm như vậy: khoét rỗng một khối hộp (Hollow selection) với độ dày thành một bước lưới, chọn các thành rồi chia chúng.

Cửa sổ **Điêu khắc**, mở bằng nút ở cuối thanh điêu khắc, chọn hình dạng của cột và cũng có thể chia vùng chọn. **Tam giác** tạo hai cột cho mỗi ô lưới, và mặt trên của tam giác luôn phẳng dù điêu khắc thế nào. **Vuông** tạo một cột cho mỗi ô, chỉ bằng nửa số brush; mặt trên của nó gập theo một đường chéo. **Lục giác** phủ brush bằng các lục giác rộng hai bước lưới, mỗi lục giác được chia thành sáu tam giác quanh tâm, nên mọi mặt trên đều phẳng.

Chọn công cụ **Điêu khắc** (`J`) và kéo trên bề mặt trong khung nhìn 3D. Vòng màu cam hiển thị cọ. Khi không chọn gì, cọ thay đổi mọi brush bên dưới; khi có vùng chọn, chỉ các brush đã chọn. Thanh ở đầu cửa sổ chọn tác dụng của một nét:

- **Nâng** và **Hạ** đưa bề mặt ra ngoài hoặc vào trong, nhiều nhất ở tâm vòng và không chút nào ở mép. Giữ `Shift` để đổi chúng cho nhau.
- **Làm mượt** làm phẳng các chỗ gồ.
- **Làm phẳng** san bề mặt về chỗ nét bắt đầu.

Trong cùng cửa sổ, **Hướng** chọn chiều di chuyển:

- **Lên** di chuyển bề mặt thẳng lên và xuống, trong một hình tròn trên mặt đất. Đây là chế độ cho địa hình.
- **Camera** di chuyển bề mặt lại gần hoặc ra xa khung nhìn, trong một khối cầu quanh con trỏ. Nhìn vào sàn, tường hoặc trần thì cọ tạo hình chỗ đó.
- **Pháp tuyến** đẩy bề mặt ra khỏi mặt nằm dưới con trỏ ở chỗ nét bắt đầu.

Hai số là **Bán kính (đơn vị)** và **Cường độ (%)**. Chỉ các góc ở phía mà nét di chuyển mới di chuyển; phía sau giữ nguyên và mỗi cột dày ít nhất một đơn vị. Góc mà các brush dùng chung sẽ di chuyển cùng nhau, nên bề mặt vẫn kín. Một nét được hoàn tác trong một bước. Trong các khung nhìn 2D, công cụ chọn như công cụ Chọn.

Khi điêu khắc xong, hãy chọn địa hình rồi chọn Hình học → **Gộp cột**, hoặc nút trong cửa sổ **Điêu khắc**. Các cột liền kề cùng tạo thành một brush lồi và trông giống nhau sẽ gộp thành một brush, nên các vùng phẳng chỉ còn vài brush lớn. Bề mặt không xê dịch, và thao tác gộp được hoàn tác trong một bước.

Để có địa hình như trong game voxel, hãy chia thành cột **Vuông** và trong cửa sổ **Điêu khắc** chọn **Khối** ở mục **Nét**. Khi đó mỗi nét di chuyển toàn bộ mặt trên của từng cột theo từng bước lưới trọn vẹn và giữ các mặt bên thẳng đứng, nên mặt đất nhô lên thành bậc. Mọi chế độ vẫn hoạt động như trước; các nét nhỏ cộng dồn cho đến khi cột lên đủ một bước. Cột có hình dạng khác được giữ nguyên.

**Tự do** không cần cột và dùng được với mọi brush. Nâng thêm đất trong một khối cầu quanh con trỏ, còn Hạ bớt đất đi, nên địa hình có thể nhô ra và khoét thành hang. Làm mượt san đều, còn Làm phẳng đưa về mức nơi nét bắt đầu. Mỗi lần chạm dựng lại trên lưới các brush trong tầm, có bo tròn, và giữ nguyên mọi thứ ngoài khối cầu, kể cả các cạnh sắc. Kết quả là các brush bình thường, nên mọi công cụ khác đều dùng được, và **Gộp cột** sau đó giảm số lượng của chúng. Lưới mịn hơn cho bề mặt mượt hơn và nhiều brush hơn.

# Vật liệu và UV
## Tải và áp dụng vật liệu
Mở phần **Vật liệu** của thẻ Mặt. **Tải thư mục** thêm một thư mục ảnh và **Tải .wad** thêm một gói texture; cả hai xuất hiện trong **Bộ sưu tập**, nơi dấu chéo dùng để gỡ bỏ. Nhập vào **Lọc vật liệu** để thu hẹp danh sách. Nhấp vào một vật liệu để đặt nó làm vật liệu hiện hành, rồi **Áp dụng cho vùng chọn** gán nó cho mọi mặt đang chọn. **Chọn** có Chọn brush dùng vật liệu này và Chọn mặt dùng vật liệu này. **Thay thế** đổi một vật liệu thành vật liệu khác trên toàn bản đồ hoặc chỉ trong vùng chọn.

Công cụ **Sơn** (`K`) gán vật liệu hiện hành cho mặt dưới con trỏ: một cú nhấp sơn một mặt, nhấp đúp sơn cả brush, và nhấp kèm `Shift` lấy vật liệu của mặt làm vật liệu hiện hành, giống như ống hút màu.

## Trình chỉnh sửa UV
Phần **UV** của thẻ Mặt hiển thị vật liệu và đường viền của mặt đang chọn. Điều khiển chuột trên vùng vẽ:
- Con lăn thu phóng quanh con trỏ. Kéo bằng nút phải hoặc nút giữa để lia. Nhấp đúp nút phải để đưa mặt vào khung hình lại; chọn một mặt khác sẽ tự động đưa mặt đó vào khung hình.
- Kéo chuột trái di chuyển vật liệu trong khi đường viền mặt giữ nguyên. Vòng tròn xoay ánh xạ, các tay nắm góc co giãn nó và tay nắm ở giữa đặt gốc cho các thay đổi này.
- Kéo chuột trái kèm `Alt` trên cạnh ô sẽ xô nghiêng, kéo chuột giữa kèm `Shift` cũng vậy.
- Mỗi lần kéo là một bước hoàn tác, và `Esc` hủy thao tác kéo đang diễn ra.

Bên dưới vùng vẽ là **Độ lệch**, **Tỷ lệ**, **Góc** và, khi trò chơi dùng đến, **Cờ nội dung**, **Cờ bề mặt** và **Giá trị bề mặt**. Các nút gồm **Đặt lại**, **Đặt lại theo thế giới**, **Khớp**, **Lật ngang**, **Lật dọc** và hai nút xoay 90°. **Căn chỉnh** xoay vật liệu theo một cạnh của mặt (nhấn lại để chọn cạnh tiếp theo, `Shift` cho cạnh trước). Bốn nút canh lề đẩy vật liệu sát vào một cạnh. **Khớp U** và **Khớp V** co giãn sao cho một số nguyên bản sao phủ kín mặt; nhấn lại để chuyển sang số tiếp theo, và giữ `Ctrl` để giữ phân chia của trim sheet.

## Phím UV
Khi đang chọn mặt, các phím mũi tên di chuyển vật liệu, `[` và `]` xoay nó 15°, `,` và `.` giảm một nửa hoặc tăng gấp đôi tỷ lệ của nó, `Ctrl+F` và `Ctrl+Alt+F` lật nó, `Shift+R` đặt lại nó, `Alt+Shift+R` đặt lại theo trục thế giới và `Ctrl+Shift+F` khớp nó với từng mặt.

**Khóa** trên dải công cụ giữ nguyên ánh xạ khi brush di chuyển, và **UV** (`U`) giữ nguyên nó khi brush xoay.

## Vật liệu công cụ
- skip: loại mặt được đánh dấu khỏi bản xuất sang engine.
- clip: tạo va chạm mà không có hình học hiển thị.
- origin: đánh dấu tâm xoay của một thực thể brush.

Có thể đổi các tên này trong thẻ **Vật liệu công cụ** của cửa sổ Xuất. Dùng skip cho những mặt bạn biết là bị che khuất.

## Loại mặt khỏi bản xuất
Chọn một mặt và bật **Loại mặt khỏi bản xuất** trong phần UV. Sọc chéo màu cam đánh dấu nó trong bản xem trước UV. Vật liệu và màu sơn của nó vẫn được lưu, và brush vẫn kín và chỉnh sửa được. Lưới kết xuất được xuất ra sẽ bỏ mặt đó trong khi vẫn giữ va chạm. Bỏ một mặt tứ giác của hình lập phương sẽ bớt hai tam giác kết xuất. Điều này không làm mặt thành hai mặt và không gộp các lưới.

**Bỏ các mặt bị che** đánh dấu các mặt bị brush tĩnh che phủ hoàn toàn, bao gồm các mặt giống hệt nhau chạm nhau và các brush bị bao kín. Nó làm việc trên các brush đang chọn, hoặc trên toàn bộ hình học thế giới đang hiện và không bị khóa khi không chọn gì. Thực thể brush, vật liệu trong suốt và brush đã có mặt bị loại sẽ được bỏ qua. Mặt bị che một phần không bị tách ra. Một lần hoàn tác khôi phục mọi thứ. Hiển thị → Hiện các mặt bị loại ẩn hoặc hiện các mặt đó trong khung nhìn của trình chỉnh sửa; việc chọn vẫn dùng toàn bộ brush.

# Màu và dải chuyển màu
Phần **Màu** của thẻ Mặt sơn các mặt bằng màu phẳng hoặc dải chuyển màu phủ lên vật liệu của chúng. Nó hữu ích để phác thảo màn chơi và cho phong cách đồ họa cách điệu. Các mặt đã sơn giữ màu thật trong khung nhìn; vùng chọn được thể hiện bằng đường viền.

## Bảng màu và màu trộn
Nhấp vào một trong mười hai ô màu để sơn các mặt đang chọn, hoặc toàn bộ brush đang chọn, bằng màu đó. **Độ đục** đặt độ đục của lần sơn tiếp theo, tính bằng phần trăm. **Trộn** có một nút màu mở bộ chọn màu; kéo trong bộ chọn và vùng chọn thay đổi theo trực tiếp, hoặc nhấn **Sơn** để áp dụng màu trộn thành một bước. **Quay lại vật liệu** gỡ màu sơn để mặt hiển thị lại vật liệu của nó. Dòng ở đầu phần cho biết đang chọn gì và nó đang mang gì.

## Bộ chọn màu
Mọi nút màu trong trình chỉnh sửa đều mở cùng một bộ chọn. Kéo trong ô vuông để chọn độ bão hòa và độ sáng, và trong dải bên cạnh để chọn sắc độ. Bên dưới là các giá trị **R**, **G** và **B** và thanh trượt **Độ đục** khi màu có độ đục. Ô hex nhận `#RRGGBB` hoặc `#RRGGBBAA`; nó chuyển sang màu đỏ khi văn bản không phải là màu hợp lệ. **Sao chép** đưa mã hex vào bộ nhớ tạm. Nhấp ra ngoài bộ chọn để đóng nó.

## Dải chuyển màu
**Dải chuyển màu** chọn Dải màu tuyến tính hoặc Dải màu tỏa tròn, và nút bên cạnh đảo ngược thứ tự màu. Mỗi điểm màu có một nút màu và **Vị trí (%)**, còn các điểm nằm giữa hai đầu có nút xóa. **Thêm điểm màu** thêm một điểm vào khoảng trống rộng nhất; một dải chuyển màu có thể có tối đa tám điểm. **Góc chuyển màu** đặt hướng của dải tuyến tính, với các nút **+45°** và **−45°**.

Dưới bản xem trước là một thanh ray có một dấu cho mỗi điểm màu. Nhấp vào thanh ray để thêm điểm, nhấp vào một dấu để mở bộ chọn màu của nó, kéo dấu để di chuyển nó, và nhấp chuột phải vào dấu để xóa nó. Ba ô màu dưới thanh ray là thiết lập sẵn: Dải màu ấm, Dải màu lạnh và Dải màu xám. Thay đổi dải chuyển màu được áp dụng trực tiếp cho vùng chọn.

Hướng của dải chuyển màu rất quan trọng: hãy đặt nó dọc theo độ dốc mái nhà hoặc dọc theo một vật thể cong, không phải ngang qua các mặt không liên quan. Kiểm tra mô hình từ cả hai phía sau khi sơn.

## Vật liệu bề mặt
**Vật liệu bề mặt** đặt **Độ nhám** (0 là bóng, 100 là mờ), **Độ kim loại** (0 cho gỗ, đá hoặc nhựa, 100 cho kim loại) và **Phát sáng** (0 đến 8; bề mặt tự phát sáng bằng màu của nó, kể cả dải chuyển màu, nhưng không chiếu sáng các vật thể xung quanh). Thay đổi được áp dụng trực tiếp cho các mặt hoặc brush đang chọn. **Dùng bề mặt mặc định** gỡ thiết lập ghi đè. Mọi thay đổi đều có thể hoàn tác.

Hiệu ứng bề mặt hiển thị trong khung nhìn 3D có chiếu sáng và trong ảnh kết xuất từ camera. Chế độ hiển thị phẳng và khung dây cố ý bỏ qua phần phản sáng. Bloom có thể thêm quầng sáng quanh các bề mặt phát sáng. Godot và glTF/GLB nhận các giá trị vật liệu, còn trình nhập của Unity chuyển độ nhám thành độ mịn. Xem `docs/surface-materials.md` để biết giới hạn của từng đích xuất.

# Tài nguyên, lớp và nhóm
## Thư viện tài nguyên
Mở thẻ **Tài nguyên**. Thêm một thư mục mô hình `.map` bằng **Thêm thư mục**; thư viện dựng sẵn luôn có mặt. Nhập vào **Lọc tài nguyên** để tìm kiếm. Nhấp vào một mô hình để chọn nó, rồi nhấp đúp, nhấn `Enter` hoặc nhấp **Chèn** để thêm nó vào vị trí con trỏ vừa đứng. Bạn cũng có thể kéo mô hình vào một khung nhìn. Brush của tài nguyên được chèn thành một nhóm để mô hình di chuyển như một khối.

**Lưu vùng chọn thành tài nguyên** ghi các brush đang chọn vào thư viện dưới dạng `.map`. Tài nguyên trong thư viện là brush thông thường có thể chỉnh sửa, không phải lưới được nhập vào.

## Lớp
Danh sách **Lớp** của thẻ Bản đồ dùng để sắp xếp các khu vực lớn. Nhập tên và nhấp **Thêm** để tạo lớp, hoặc nhấn `Enter` để đổi tên lớp hiện hành. Mỗi lớp có thể được ẩn, khóa hoặc loại khỏi bản xuất. **Di chuyển đến đây** đưa vùng chọn vào lớp hiện hành. Menu ngữ cảnh trong khung nhìn cũng có Chuyển sang lớp, Đặt làm lớp hiện hành, Ẩn lớp và Cô lập lớp.

## Nhóm và nhóm liên kết
`Ctrl+G` gom vùng chọn thành nhóm và `Ctrl+Shift+G` rã nhóm. Phần **Nhóm** của thẻ Bản đồ liệt kê các nhóm và có **Mở**, **Đổi tên**, **Hợp nhất**, **Rã nhóm**, **Lấy ra** và **Tách ra**. Nhóm liên kết là một tập bản sao dùng chung các chỉnh sửa: `Ctrl+Shift+D` tạo bản sao liên kết, Hình học → Đồng bộ nhóm liên kết cập nhật các bản sao, và **Hủy liên kết** làm một bản sao trở nên độc lập. Trong thẻ Thực thể, một thuộc tính có thể được giữ riêng cho một bản sao khi các nhóm liên kết đồng bộ.

## Tỷ lệ chung
Tỷ lệ xuất mặc định là 32 đơn vị bản đồ mỗi mét. Điểm đánh dấu vị trí xuất phát của người chơi dựng sẵn cao 64 đơn vị, làm mốc tham chiếu 2 mét. Hãy so sánh cửa ra vào, xe cộ, đồ đạc và cây cối với nó. Chỉ đổi tỷ lệ xuất khi có chủ đích và giữ các tài nguyên liên quan nhất quán.

# Thực thể và lối chơi
Các định nghĩa thực thể thông dụng được dựng sẵn và luôn có sẵn: đèn, vị trí xuất phát của người chơi, cửa, nút bấm, bệ di chuyển, trigger, đạo cụ tĩnh, âm thanh và điểm đường đi. Tải một tệp FGD của dự án bằng **Định nghĩa thực thể** trong thẻ Thực thể sẽ thay thế các lớp cùng tên và giữ các lớp dựng sẵn còn lại.

## Đặt thực thể
Trong trình duyệt **Thực thể** của thẻ Thực thể, nhập vào **Lọc lớp** và chọn một lớp. Với lớp điểm như đèn, nhấp **Đặt** hoặc nhấp đúp vào lớp; `Insert` đặt một thực thể tại tâm vùng chọn. Với lớp brush như cửa hoặc trigger, chọn các brush rồi nhấp **Tạo**. Cũng có thể kéo các lớp vào một khung nhìn.

## Thuộc tính
Danh sách thuộc tính chỉnh sửa thực thể đang chọn. Các thực thể cùng lớp được chỉnh sửa cùng lúc. **Thêm thuộc tính** thêm một khóa mà định nghĩa có thể không biết; **Mặc định** có thể đưa các thuộc tính hiện có về giá trị mặc định hoặc thêm những thuộc tính còn thiếu. Các thuộc tính mà lớp định nghĩa nhưng thực thể chưa có được hiển thị mờ; nhập một giá trị sẽ thêm chúng. Dùng targetname và target để liên kết các thực thể.

## Bản xuất làm được gì
Các chỉ thị của Godot tạo nút gốc, thân va chạm và vùng trigger. Unity và Unreal hỗ trợ các loại đèn thông dụng dưới dạng gốc, và Unreal còn tạo actor PlayerStart. Dữ liệu khác được giữ lại trong phạm vi trình nhập của đích xuất cho phép.

Riêng một thực thể cửa không tự thêm hành vi trò chơi. Chuyển động, phản ứng với trigger, bộ điều khiển người chơi và script của dự án vẫn cần được làm trong engine. Defold xuất thuộc tính thực thể và đối tượng va chạm; Flax đặt mô hình và thực thể chung. Hãy kiểm tra kết quả trong engine trước khi xây dựng lối chơi dựa trên nó.

# Hoạt ảnh
Heron Brush có thể xem trước chuyển động đơn giản và chỉnh sửa hoạt ảnh khung khóa của thực thể brush, rồi xuất chúng sang nhiều engine.

## Xem trước hoạt ảnh
Bật **Xem trước hoạt ảnh** trong menu Hiển thị, menu Hoạt ảnh, menu Cài đặt lưới hoặc phần Hiển thị của thẻ Bản đồ. Nó phát chuyển động xoay của func_rotate và chuyển động qua lại của func_shuttle khi start_on được bật, dùng các thuộc tính trục, tốc độ, hướng, hành trình và thời gian chờ của thực thể, và nó phát các clip khung khóa được đặt Phát khi bắt đầu. Các tài nguyên quạt và cối xay gió đã có sẵn bộ phận quay.

Tắt bản xem trước sẽ khôi phục tư thế ban đầu. Một cú nhấp trong khung nhìn sẽ dừng phát trước khi chọn hoặc chỉnh sửa; điều hướng camera vẫn hoạt động. Bản xem trước không thay đổi bản đồ, lịch sử hoàn tác hay bản xuất, và không chạy vật lý hay logic trò chơi.

## Trình chỉnh sửa khung khóa
Mở nó bằng Hoạt ảnh → Hoạt ảnh khung khóa hoặc từ menu Cài đặt lưới. Nó được gắn bên dưới các khung nhìn, hoặc trong cửa sổ 3D khi cửa sổ đó đang tách riêng. Biểu tượng cửa sổ ở tiêu đề của nó đưa nó vào một cửa sổ riêng; khung hình hiện tại và lịch sử hoàn tác được giữ nguyên. **Chỉnh sửa hoạt ảnh** chuyển các khung nhìn sang tư thế hoạt ảnh và **Chỉnh sửa bản đồ** quay về hình học ở trạng thái nghỉ. Ở chế độ hoạt ảnh, mỗi khung nhìn hiển thị **Chỉnh sửa hoạt ảnh** và số khung hình ở phía dưới.

Chọn một thực thể brush, hoặc một brush hay nhóm brush, để tạo hoạt ảnh cho nó. Một nhóm brush phẳng dùng chung một hoạt ảnh và một tâm xoay; các nhóm chứa nhiều thực thể cần các hoạt ảnh riêng.

## Clip
Bộ chọn **Hoạt ảnh** chuyển giữa các clip có tên. Nhập tên vào **Tên hoạt ảnh** và nhấp **Lưu thành** để sao chép clip hiện tại dưới tên đó; **Xóa** xóa một clip. **Tịnh tiến** tạo clip `translate` không có xoay. **Hoạt ảnh cửa** tạo các clip `open` và `close` mượt với một vòng xoay 90 độ quanh tâm xoay hiện tại; hãy di chuyển tâm xoay đến bản lề trước. **Phát khi bắt đầu** làm clip phát khi màn chơi bắt đầu; thiết lập sẵn cho cửa tắt tùy chọn này.

## Điều khiển phát
Các nút điều khiển phát là **Bắt đầu**, **Khóa trước**, **Phát lại**, **Khóa tiếp theo** và **Kết thúc**. **Khung hình** hiển thị và đặt khung hình hiện tại, còn **Thời lượng (s)** đặt độ dài clip. Chế độ là **Một lần**, **Lặp** hoặc **Qua lại**.

## Dòng thời gian
Dòng thời gian có một thước với đầu phát và một làn cho **Vị trí** (X, Y và Z được đặt khóa cùng nhau) và cho từng góc xoay, RX, RY và RZ. Nhấp hoặc kéo trên thước để di chuyển đầu phát và xem trước tư thế đó trong mọi khung nhìn. Kéo bằng nút giữa để lia dòng thời gian. Kéo các khóa để dời chúng theo thời gian; `Shift` thêm vào vùng chọn; nhấp đúp vào một làn để chèn khóa; `Ctrl`+con lăn để thu phóng. **Đường cong** hiển thị các đường cong giá trị thay cho các làn.

Các nút phía trên là **Thêm khóa**, **Gỡ bỏ**, **Sao chép**, **Dán** và **Nhân bản**. **Bám theo khung hình** giữ khóa ở khung hình nguyên và **Khớp** hiển thị toàn bộ clip. **Định thời lại khóa** mở **Tỷ lệ thời gian**, **Độ lệch (khung hình)**, **Co giãn từ đầu phát**, **Kéo dài thời lượng** và **Định thời lại vùng chọn**.

Bảng khóa bên cạnh dòng thời gian chỉnh sửa khóa đang chọn: **Khung hình**, **Giá trị** (hoặc ba tọa độ của khóa Vị trí), kiểu nội suy **Tuyến tính**, **Giữ nguyên** hoặc **Bezier**, **Độ dốc vào** và **Độ dốc ra**, và **Tiếp tuyến mượt**.

## Phím trong trình chỉnh sửa
Khi dòng thời gian nhận bàn phím: `Space` phát hoặc tạm dừng, mũi tên trái và phải bước một khung hình, `Ctrl+A` chọn mọi khóa, `Ctrl+C`, `Ctrl+X` và `Ctrl+V` sao chép, cắt và dán khóa, `Ctrl+D` nhân bản chúng, `Delete` xóa chúng, `Ctrl+Z` hoàn tác và `Esc` hủy thao tác kéo. Bên ngoài dòng thời gian, `Esc` thoát chế độ hoạt ảnh. Hoàn tác giữ nguyên mức thu phóng, vùng chọn và đầu phát như cũ.

## Tạo tư thế trong khung nhìn
Hàng bên dưới dòng thời gian có **Vỏ hành** với số lượng và **Khoảng cách**, hiển thị các tư thế lân cận dưới dạng bóng mờ; tọa độ **Tâm xoay**; **Tự động tạo khóa**; và các công cụ tạo tư thế **Tịnh tiến**, **Xoay** và **Tâm xoay**.

- **Thêm khóa** bật Tự động tạo khóa và tạm dừng phát. Khi bật Tự động tạo khóa và chọn **Tịnh tiến**, kéo một trục màu của tay nắm thực thể đang chọn trong khung nhìn 3D để đặt khóa vị trí của nó tại khung hình hiện tại.
- Trong khung nhìn 2D, kéo brush đang chọn hoặc một thành viên trong nhóm của nó để đặt khóa tịnh tiến theo hai trục đang thấy, có bám lưới. Không cần bật Tự động tạo khóa cho việc này.
- Với **Xoay**, kéo brush trong khung nhìn 2D quanh dấu tâm xoay màu cam của nó. Phía trên đặt khóa RZ, Phía trước đặt khóa RY và Bên cạnh đặt khóa RX, bám theo các góc lưới.
- Với **Tâm xoay**, kéo tâm xoay màu vàng trong bất kỳ khung nhìn 2D nào. Nó bám theo lưới và giữ nguyên chiều sâu. Khóa và hình học ở trạng thái nghỉ không thay đổi.

Đường màu lục lam trong 2D và 3D thể hiện hành trình của tâm xoay. Nhấp vào một điểm trên đường để chọn thời điểm và khóa của nó, hoặc kéo nó để đổi vị trí; khi bật Tự động tạo khóa, bạn cũng có thể kéo các điểm trên đường trong khung nhìn 3D. Nhãn hiển thị số khung hình và các đoạn nét đứt đánh dấu bước nhảy. `Esc` hủy bất kỳ thao tác kéo nào trong số này, và một lần hoàn tác đảo ngược cả một lần kéo.

Hoạt ảnh mới bắt đầu ở trạng thái nghỉ với các khóa tại thời điểm 0. Tạo chuyển động bằng cách thay đổi tư thế ở một khung hình khác. Với đường hình chữ L sắc cạnh, hãy đặt khóa ở điểm đầu, góc và điểm cuối, và dùng Tuyến tính với Qua lại.

## Kết nối nút bấm với hoạt ảnh
Chọn một func_button, trigger_multiple hoặc trigger_once. Thẻ Thực thể hiển thị **Kích hoạt hoạt ảnh** phía trên các thuộc tính. Chọn một đích trong danh sách, hoặc nhấp **Chọn đích** rồi nhấp vào một đối tượng có hoạt ảnh trong bất kỳ khung nhìn nào; `Esc` hủy việc chọn. Đích hoặc clip bị thiếu được báo bên dưới các điều khiển. Liên kết thực thể hiển thị mũi tên tới đích của chúng khi bật hiển thị liên kết.

**Hành động** chọn việc mà lần kích hoạt thực hiện: **Phát** bắt đầu một clip có tên hoặc hoạt ảnh đang chọn; **Bật/tắt** luân phiên `open` và `close` khi chúng tồn tại, nếu không thì khởi động lại hoạt ảnh đang chọn; **Mở** và **Đóng** bắt đầu các clip đó; **Tạm dừng** giữ nguyên tư thế, **Tiếp tục** chạy tiếp và **Dừng** khôi phục tư thế ban đầu. **Chỉ một lần** ghi đè giá trị mặc định của lớp. **Độ trễ lặp lại (s)** đặt thời gian ngắn nhất giữa các lần kích hoạt; nó không làm trễ lần đầu tiên.

**Thử trigger** chạy hành động trong bản xem trước mà không chỉnh sửa bản đồ hay thêm bước hoàn tác. **Đặt lại xem trước** khôi phục các tư thế ban đầu và xóa trạng thái một lần và thời gian chờ. Việc này không mô phỏng va chạm của người chơi hay script trò chơi.

## Hoạt ảnh trong bản xuất
Godot xuất các tài nguyên AnimationPlayer và AnimationLibrary gốc, bao gồm đường cong và va chạm chuyển động. Một CharacterBody3D đi vào vùng của nút bấm sẽ kích hoạt đích của nút đó. Các chỉnh sửa trên những clip này trong Godot không quay về bản đồ và sẽ bị thay thế ở lần xuất tiếp theo.

Bản xuất raylib thêm `<map>_animation.h` với trình phát viết bằng C và `<map>_preview.c` với bản xem trước camera tự do; làm theo `<map>_raylib.md` để biên dịch nó. Trong bản xem trước, `SPACE` tạm dừng, `R` khởi động lại các clip mặc định và `N` xoay vòng qua các clip có tên. Defold nhúng việc phát vào script của thực thể với các thông điệp `heron_*_animation` có tài liệu. Build Level của Unity thêm `HeronBrushAnimation` và một thân động học. Bản xuất Bevy bao gồm `heron_animation.rs` và hướng dẫn. Godot, Unity, Defold, Bevy và raylib giữ liên kết trigger, cờ một lần và độ trễ lặp lại. Trò chơi của bạn vẫn phải đăng ký trình phát và kết nối các nút bấm của riêng nó với API của từng runtime.

## Ví dụ Motion lab
Mở `motion_lab` từ mục Ví dụ trên màn hình chào mừng. Nó có một bệ di chuyển theo đường chữ L, một thang nâng, một bệ xoay và một cửa có bản lề cùng các nút bấm. Mở `godot/project.godot` nằm cạnh nó trong Godot và nhấn `F5`. WASD và chuột di chuyển người chơi, Space để nhảy, 1 đến 4 đến các trạm và R khởi động lại trạm hiện tại. Bước lên một tấm đệm màu để kích hoạt đích của nó. README đi kèm giải thích cách xuất lại.

# Xuất sang engine
Tệp → Xuất mở cửa sổ Xuất. Các engine được liệt kê dọc bên trái kèm logo: Godot, Defold, three.js, Bevy, raylib, Unity, Unreal Engine, Flax Engine, Cocos Creator, Wavefront .obj, glTF .gltf, GLB .glb, và cuối cùng là `.map`. Trang bên phải cho biết đích xuất sẽ nhận được gì. **Ghi** ghi nó ra, **Kiểm tra** dựng màn chơi mà không ghi và liệt kê các vấn đề, còn **Đóng** đóng cửa sổ.

Xuất tách biệt với lưu: tệp `.map` vẫn là bản nguồn có thể chỉnh sửa của bạn. Mỗi điều khiển lưu cài đặt của engine ngay khi thay đổi, nên mỗi engine giữ lựa chọn riêng và cửa sổ mở lại đúng như khi bạn rời đi.

## Tùy chọn chung
- **Tỷ lệ**: **Đơn vị trên mỗi mét**, mặc định là 32.
- **Nội dung cần ghi**: **Hình học va chạm**.
- **Tối ưu hóa**: **Gom lưới tĩnh theo ô 1024 đơn vị**, **Bỏ các mặt bị vùi bên trong hình học**, **Gộp các mặt cùng mặt phẳng** và **Gộp hình va chạm**.
- **Đường dẫn dự án**: nơi màn chơi và tài nguyên của nó được đặt trong dự án.
- **Vật liệu công cụ**: tên của các vật liệu clip, skip và origin.
- **Thêm**: các công tắc riêng cho engine như **Đặt tên nút theo**, **Dựng lại lớp và nhóm thành nút cha**, **Đảo chiều cuốn tam giác**, lật tọa độ texture và bộ UV thứ hai cho ánh sáng nướng sẵn với **Texel lightmap**.
- **Theo kịp bản đồ**: **Ghi lại mỗi khi lưu bản đồ**. Tùy chọn này dùng được sau khi engine đã được xuất một lần, vì lần xuất đó cho biết nơi cần ghi.

Ô tĩnh gán brush thế giới theo tâm của chúng và kết hợp chúng theo vật liệu trong mỗi ô 1024 đơn vị, để engine có thể loại bỏ những phần nhỏ hơn của bản đồ. Va chạm được giữ nguyên và các thực thể lối chơi vẫn tách riêng. Cách này có thể tạo nhiều lệnh vẽ hơn so với một lưới thế giới duy nhất, nên mặc định nó bị tắt. Các bản xuất OBJ, glTF và GLB độc lập không dùng ô.

## Ghi chú về đích xuất
- Godot: đường dẫn `res://` của texture và vật liệu, lớp vật lý của trigger và UV2 cho lightmap. Hãy chọn thư mục xuất nằm trong dự án Godot.
- Defold: thư mục tài nguyên của asset, vật liệu của mô hình và texture sampler, vốn phải tồn tại trong vật liệu đã chọn.
- Unity: lưới OBJ và một trình nhập cho editor; hãy xuất vào thư mục Assets của dự án.
- Unreal Engine: một script nhập được tạo sẵn và đường dẫn nội dung, thường là /Game/<level>.
- Flax Engine: đường dẫn nội dung của mô hình, tương đối với thư mục Content của dự án.
- three.js, Bevy, raylib và Cocos Creator: cảnh hoặc mã cho các runtime đó, kèm hướng dẫn nằm cạnh bản xuất.
- glTF / GLB: hình học di động cho các công cụ tương thích; GLB là dạng nhị phân. OBJ: hình học và vật liệu cho các công cụ dựng hình.
- `.map`: **Ghi dạng** chọn Valve 220, Tiêu chuẩn, Quake 2 hoặc Quake 2 (Valve). Cửa sổ cho biết phương ngữ được chọn sẽ mất gì.

Đọc các cảnh báo sau khi ghi, chẳng hạn texture bị thiếu. **Kết quả kiểm tra** liệt kê chúng trong cửa sổ.

# Liên kết trực tiếp: Godot, Unity, Defold và Flax
Liên kết trực tiếp gửi các chỉnh sửa chưa lưu từ Heron Brush sang trình chỉnh sửa của engine trong khi bạn làm việc. Liên kết chỉ có một chiều: chỉnh sửa trong engine không quay lại. Mỗi cửa sổ bản đồ chạy được một liên kết trực tiếp. Hãy dừng nó trước khi đổi engine, và khởi động lại để áp dụng tùy chọn xuất đã thay đổi. Các điều khiển nằm trong cửa sổ Xuất, trên trang của engine, trong thẻ **Liên kết trực tiếp với**.

Chỉ báo trên thanh tiêu đề hiển thị trạng thái của liên kết; nhấp vào nó để dừng liên kết. Đóng hoặc thay bản đồ sẽ dừng việc gửi dữ liệu. Sau khi nhập thất bại, hãy sửa lỗi và khởi động lại liên kết.

## Godot
1. Xuất một lần vào một dự án Godot.
2. Nhấp **Cài addon Godot**.
3. Bật Heron Brush Live trong Project Settings → Plugins của Godot.
4. Nhấp **Bắt đầu liên kết trực tiếp** và gắn bản đồ từ dock của Godot vào một cảnh 3D đang mở.

Addon chỉ thay thế nhánh được tạo ra của nó và giữ nguyên vị trí gắn cùng các nút khác. Hãy tự lưu cảnh Godot, và giữ script và nút của riêng bạn bên ngoài nhánh được tạo ra.

## Unity
1. Xuất một lần vào thư mục Assets của dự án Unity.
2. Nhấp **Cài addon Unity** và để Unity biên dịch nó.
3. Mở Window → Heron Brush → Live Link trong Unity.
4. Bắt đầu liên kết và gắn bản đồ vào cảnh Unity đang hoạt động.

Hãy để cửa sổ live của Unity luôn mở. Cập nhật tạm dừng trong Play Mode và Prefab Mode. Chỉ phần con Generated của đối tượng bản đồ bị thay thế. Không chỉnh sửa các tệp được tạo ra.

## Defold
1. Xuất một lần vào dự án Defold của bạn và giữ dự án mở trong Defold.
2. Nhấp **Bắt đầu liên kết trực tiếp**. Không cần addon.
3. Mở heron_brush_live/preview.collection trong Defold, hoặc tham chiếu nó từ cảnh của riêng bạn.

Liên kết ghi vào một thư mục được tạo riêng và không đụng đến bản xuất cuối cùng cũng như game.project. **Nạp lại nóng trò chơi đang chạy** cũng cập nhật trò chơi được khởi chạy từ Defold; thêm đối tượng hoặc thay đổi va chạm có thể cần khởi động lại màn chơi. Kết nối thất bại sẽ được tự động thử lại.

## Flax
Xuất vào thư mục Content của dự án và nhấp **Cài addon Flax**. Để Flax biên dịch nó, bắt đầu liên kết, rồi chọn Tools → Heron Brush: attach live map trong cảnh Flax đang mở. Gốc được tạo ra giữ nguyên vị trí và các đối tượng khác không bị đụng đến. Hãy tự lưu cảnh. Cập nhật tạm dừng trong Play Mode. Tools → Heron Brush: stop live map tạm dừng việc nhận dữ liệu.

# Phiên cộng tác trực tiếp
Cộng tác → Phiên trực tiếp mở cửa sổ để mở phiên hoặc tham gia cùng một người dùng Heron Brush khác. Tính năng này chia sẻ việc chỉnh sửa bản đồ giữa nhiều người; nó tách biệt với liên kết trực tiếp tới engine.

## Mở phiên và tham gia
Nhập **Tên của bạn**. Để mở phiên, chọn **Mở một phiên**, nhấp **Bắt đầu phiên** và gửi thông tin từ **Sao chép thông tin kết nối** cho khách của bạn. Để tham gia, chọn **Tham gia phiên**, dán thông tin và nhấp **Nối**. Việc tham gia sẽ thay bản đồ đang mở bằng bản đồ của chủ phiên, nên hãy lưu công việc của bạn trước.

Bản dựng tiêu chuẩn hoạt động trong mạng cục bộ, và **Cổng mạng cục bộ** đặt cổng. Phiên qua internet (**Kết nối qua internet**) cần bản dựng có tính năng mạng tùy chọn. Người tham gia thấy dấu camera và tên của nhau trong khung nhìn 3D.

Đóng cửa sổ thì phiên vẫn tiếp tục chạy; chỉ báo trên thanh tiêu đề hiển thị trạng thái của nó. **Dừng phiên** hoặc **Rời phiên** kết thúc phiên. Khi hai người cùng sửa một đối tượng, thứ tự của chủ phiên quyết định phiên bản cuối cùng, và hoàn tác cũng là một chỉnh sửa chung. Hãy làm việc ở các khu vực riêng khi có thể, và lưu các điểm mốc.

# Hiển thị 3D và màn hình thứ hai
## Chế độ hiển thị
Phần **Hiển thị** của thẻ Bản đồ chuyển đổi những gì khung nhìn vẽ: Brush, Thực thể điểm, Mô hình thực thể, Tên lớp thực thể, Đường viền nhóm, Patch, Vật liệu, Hiện các mặt bị loại, Xem trước hoạt ảnh, Khung dây, Cạnh và Đường viền thực thể brush. Khung dây vẽ các cạnh tam giác, bao gồm đường chéo và cạnh phía sau bề mặt, không có vật liệu. Đây là chế độ gỡ lỗi tạm thời và không ảnh hưởng đến bản xuất hay ảnh kết xuất.

Chiếu sáng, bóng đổ, che khuất môi trường, phản chiếu, bloom và làm mịn cho khung nhìn 3D nằm trong Tùy chọn → Bộ kết xuất. Đây là các cài đặt xem trước; ảnh cuối cùng dùng cài đặt riêng của camera kết xuất.

## Cửa sổ 3D riêng
Hiển thị → Khung nhìn 3D trong cửa sổ riêng đưa khung nhìn 3D vào một cửa sổ riêng, mà bạn có thể đặt trên màn hình khác. Nó dùng chung camera, tài liệu và công cụ, còn các khung nhìn phẳng chiếm chỗ trong cửa sổ chính. Menu của nó có Vật liệu, Hiển thị khối đặc và Khung dây; menu con Hiển thị với Hiện các mặt bị loại, Xem trước hoạt ảnh, Cạnh, Đường viền nhóm, Thực thể điểm, Mô hình thực thể và Đường viền thực thể brush; Lấy nét vào vùng chọn; Phía trước, Bên cạnh và Phía trên, xoay camera quanh vùng chọn (hoặc một điểm phía trước camera); Đặt camera từ khung nhìn 3D; Toàn màn hình (`F11`); Kết xuất ảnh; và Quay lại cửa sổ chính. Đóng cửa sổ sẽ gắn khung nhìn trở lại.

## Bầu trời thủ tục
Bầu trời thủ tục được bật mặc định trong khung nhìn 3D và có thể tắt trong Tùy chọn → Bộ kết xuất → Ánh sáng. Camera kết xuất có công tắc **Bầu trời thủ tục** riêng, được lưu cùng bản đồ. Màu bầu trời, đường chân trời và mặt đất lấy từ cài đặt môi trường, và mặt trời theo ánh sáng của cảnh. Bầu trời không có hình học và không được xuất.

# Camera kết xuất và ảnh PNG
Tệp → Kết xuất ảnh mở cửa sổ kết xuất: camera và cài đặt của nó ở bên trái và bản xem trước trực tiếp ở bên phải, được vẽ lại khi bạn thay đổi chúng. Camera được lưu trong tệp `.map` ở một lớp Render cameras chỉ dành cho trình chỉnh sửa, lớp này bị loại khỏi bản xuất sang engine.

## Đặt camera
Căn khung hình trong khung nhìn 3D và nhấn `Ctrl+Alt+0` (hoặc `Ctrl+Alt+Num0`), hoặc nhấp **Đặt camera từ khung nhìn 3D**. Nếu chưa chọn camera nào, camera đã lưu đầu tiên được cập nhật; nếu chưa có camera nào, một camera mới được tạo. Thao tác này có thể hoàn tác và giữ nguyên cài đặt đầu ra và chiếu sáng. **Chọn camera trong bản đồ**, **Tải camera từ bản đồ** và **Đưa khung nhìn 3D đến camera** làm theo chiều ngược lại. **Khung camera trong khung nhìn 3D** hiển thị ranh giới đầu ra, và **Điều khiển camera từ khung nhìn 3D** cho phép điều hướng thông thường di chuyển camera đã lưu.

## Cài đặt
- **Vị trí camera**: X, Y, Z, góc ngang và góc ngẩng.
- **Đầu ra và ống kính**: **Thiết lập sẵn** (Full HD, Vuông, Dọc), chiều rộng, chiều cao, số mẫu, **Góc nhìn dọc** và **Mặt phẳng cắt xa**.
- **Chiếu sáng**: Ánh sáng ban ngày, Hoàng hôn hoặc Trời u ám, hoặc tự đặt độ phơi sáng, mặt trời, màu bầu trời, đường chân trời và mặt đất, **Bầu trời thủ tục**, **Bóng đổ**, **Che khuất môi trường** và **Bloom**.
- **Độ sâu trường ảnh**: **Bật độ sâu trường ảnh**, khoảng cách lấy nét, vùng nét và bán kính làm mờ. **Lấy nét vào vùng chọn** lấy nét vào tâm của đối tượng đang chọn ở phía trước camera.

**Lưu camera và cài đặt** lưu biểu mẫu vào bản đồ; hãy lưu bản đồ để giữ nó trên đĩa.

## Kết xuất
**Kết xuất PNG** hỏi nơi lưu và bắt đầu kết xuất lũy tiến, hình ảnh được bổ sung dần từng mẫu trong bản xem trước. Nhiều mẫu hơn cho kết quả đẹp hơn nhưng mất nhiều thời gian hơn. **Hủy kết xuất** dừng lại mà không ghi tệp. Bộ kết xuất dùng quy trình raster của trình chỉnh sửa, không dùng path tracing. Ảnh PNG có nền đục, và hoạt ảnh không được kết xuất. Ảnh rất lớn có thể làm trình chỉnh sửa khựng lại trong chốc lát.

# Tùy chọn
Tệp → Tùy chọn (`Ctrl+,`) mở phần cài đặt. Thay đổi có hiệu lực ngay và được lưu lại. Thanh bên liệt kê Tổng quan, Trò chơi, Hiển thị, Màu sắc, Chuột, Bàn phím và Bộ kết xuất. Ô tìm kiếm ở trên cùng tìm một cài đặt theo tên hoặc mô tả trên bất kỳ trang nào. Mỗi cài đặt đã thay đổi có nút riêng để khôi phục giá trị mặc định, và **Khôi phục mặc định** đặt lại trang hiện tại.

## Các trang
- **Tổng quan**: các cài đặt thông dụng gom lại: Giao diện (Độ đục bảng (%), Bảng màu giao diện, Bố cục, Ngôn ngữ), Chỉnh sửa, Hiển thị và Hướng dẫn sử dụng.
- **Trò chơi**: trò chơi, vị trí các tệp của nó, và tên cộng tác của bạn. Trò chơi quyết định định dạng bản đồ để ghi, định nghĩa thực thể để tải và vị trí vật liệu. Cấu hình trò chơi được đọc từ thư mục `games` trong thư mục cấu hình (`~/.config/heron-brush/games` trên Linux và macOS, `%LOCALAPPDATA%\heron-brush\games` trên Windows) và từ thư mục `games` cạnh ứng dụng, nơi các gói cài đặt đặt cấu hình đi kèm.
- **Hiển thị**: Bố cục, Bảng màu giao diện, Ngôn ngữ, trục thế giới, độ đục lưới, kích thước ô kiểm tra rò rỉ và các khóa ánh xạ.
- **Màu sắc**: màu của khung nhìn, hình học và tay nắm.
- **Chuột**: tốc độ và đảo chiều khi nhìn, lia và di chuyển, tốc độ bay, góc nhìn, Liên kết các khung nhìn 2D và kích thước tay nắm.
- **Bàn phím**: sơ đồ phím, Heron Brush hoặc Blender.
- **Bộ kết xuất**: tô bóng chân thực, đường viền vùng chọn, ánh sáng, bóng đổ, che khuất môi trường, phản chiếu, bloom và bề mặt.

## Ngôn ngữ
**Ngôn ngữ** có Theo hệ thống và mọi ngôn ngữ đã được dịch. Giao diện thay đổi ngay. Hướng dẫn mở bằng ngôn ngữ giao diện nếu có bản dịch, nếu không thì bằng tiếng Anh.

# Plugin và MCP
## Plugin
Plugin mở rộng trình chỉnh sửa bằng script Rhai. Tệp → Plugin mở cửa sổ Plugin. **Cài plugin** cài một tệp `.rhai` và **Cài thư mục plugin** cài một thư mục chứa `main.rhai`; siêu dữ liệu `plugin.toml` tùy chọn mô tả plugin và cài đặt của nó. Plugin được nhóm trong **Plugin đang hoạt động** và **Plugin không hoạt động**, và **Bật plugin** bật hoặc tắt một plugin; trạng thái này được giữ sau khi khởi động lại. **Tải lại plugin** đọc lại script từ đĩa và đặt lại giá trị trong bảng của chúng.

Plugin đang hoạt động cũng xuất hiện trong phần **Plugin** của thẻ Bản đồ cùng cài đặt, nút hành động (**Chạy**) và đầu ra của chúng. Một hành động làm việc trên vùng chọn, hoặc trên mọi thứ khi không chọn gì, và là một bước hoàn tác. Plugin Selection inspector đi kèm hiển thị số lượng và các hành động đưa lên và đưa xuống.

## MCP
Cộng tác → Phục vụ cho agent (MCP) cho phép một agent AI hoặc công cụ khác điều khiển trình chỉnh sửa đang mở qua Model Context Protocol. Cửa sổ hiển thị địa chỉ cục bộ cùng nút **Sao chép**, và **Dừng máy chủ** kết thúc nó; trong khi chạy, thanh tiêu đề hiển thị **MCP đang hoạt động**. Tùy chọn dòng lệnh `--mcp` chạy cùng giao diện đó mà không có cửa sổ. Ứng dụng khách có thể khám phá các công cụ cho hình học, tài nguyên, thuộc tính, hoạt ảnh, xuất và phiên. MCP và plugin của trình chỉnh sửa là hai giao diện khác nhau.

# Dòng lệnh
Chương trình là `heron-brush`. Khi không có tùy chọn, nó mở màn hình chào mừng.

## Mở tệp
`heron-brush map.map [entities.fgd] [materials/]` mở một bản đồ, tùy chọn kèm định nghĩa thực thể và thư mục vật liệu. Một đối số rỗng sẽ bỏ qua mục tương ứng, ví dụ `heron-brush map.map "" materials/`. Nếu một trình chỉnh sửa đang chạy, bản đồ sẽ mở trong đó; `--new-instance` thay vào đó mở một cửa sổ riêng.

## Lệnh
- `--help` hoặc `-h`: in cách dùng. `--version` hoặc `-V`: in phiên bản.
- `--mcp`: phục vụ MCP qua đầu vào và đầu ra chuẩn, không có cửa sổ.
- `--merge-map <base> <ours> <theirs> [<output>]`: hợp nhất ba chiều các tệp `.map` cho git. Đầu ra mặc định là ours. Xung đột được in ra và cho mã thoát khác không.
- `--obj-to-map <in.obj> <out.map> [size] [thickness] [snap] [material]`: chuyển một lưới thành brush. Size là độ dài cạnh dài nhất tính bằng đơn vị bản đồ (mặc định 256), snap là lưới cho các góc (0 là không có), và các từ `solid` hoặc `layered` tạo hộp lưới hoặc tấm lồi thay cho nêm mỏng.
- `--render-preview <in.map> <out.png> [size]`: ghi ảnh thu nhỏ PNG của bản đồ, mặc định 512 điểm ảnh.
- `--run-script <script.rhai> <in.map> [out.map] [key=value ...]`: chạy script plugin trên bản đồ với mọi thứ được chọn và ghi kết quả (mặc định ghi đè lên tệp đầu vào). `key=value` trả lời các cài đặt của plugin.

# Khắc phục sự cố
## Khi có gì đó trông không đúng
- Thiếu vật liệu: kiểm tra thư mục hoặc gói vật liệu và đường dẫn dự án của bản xuất.
- Kích thước mô hình sai: so sánh với điểm đánh dấu người chơi 64 đơn vị và tỷ lệ xuất.
- Hướng dải chuyển màu sai: kiểm tra góc chuyển màu và hướng của mặt.
- Cảnh trong engine trống: đọc các cảnh báo khi xuất và làm theo hướng dẫn nhập của đích xuất.
- Liên kết trực tiếp đang chờ: kiểm tra addon đã được cài, cảnh đã được gắn và cập nhật không bị tạm dừng.
- Chỉ có đường, không có mặt: tắt Khung dây trong phần Hiển thị của thẻ Bản đồ, hoặc chọn Vật liệu hoặc Hiển thị khối đặc trong menu của cửa sổ 3D.
- Đối tượng biến mất khỏi khung nhìn: `Ctrl+Shift+I` hiện các đối tượng bị ẩn; kiểm tra các lớp và bộ lọc hiển thị.
- Phím không có tác dụng: nhấp vào khung nhìn trước để nó nhận bàn phím, và kiểm tra sơ đồ phím trong Tùy chọn → Bàn phím.
- Một chỉnh sửa thất bại: hoàn tác, đơn giản hóa hình học, và dùng **Kiểm tra bản đồ** và **Kiểm tra rò rỉ** trong phần **Vấn đề** của thẻ Bản đồ.
- Bản đồ không mở được từ trình quản lý tệp: có thể nó đã được chuyển cho một trình chỉnh sửa đang chạy; hãy tìm cửa sổ đó.

## Cài đặt và bản sao lưu
Tùy chọn, bản đồ gần đây, ngôi sao, cài đặt xuất và plugin nằm trong thư mục cấu hình: `~/.config/heron-brush` trên Linux và macOS, và `%LOCALAPPDATA%\heron-brush` trên Windows. Bản sao lưu tự động nằm trong thư mục `autosave` cạnh mỗi bản đồ.

## Báo lỗi
Hãy kèm phiên bản trình chỉnh sửa (Trợ giúp → Giới thiệu), hệ điều hành của bạn, một bản đồ nhỏ thể hiện vấn đề và các bước chính xác. Khung đầu ra (`Ctrl+4`) thường cho biết điều gì đã sai. Hãy lưu một bản sao trước khi thử nghiệm.

## Sử dụng hướng dẫn này
Chọn một chương trong mục lục bên trái. `Ctrl+F` hiển thị mục lục và chuyển đến ô tìm kiếm; `Esc` xóa nội dung tìm kiếm, hoặc đóng hướng dẫn khi ô tìm kiếm trống. **Nội dung** ẩn hoặc hiện danh sách chương và **Cỡ chữ** thay đổi kích thước phông chữ; cả hai đều được ghi nhớ. Danh sách ngôn ngữ ở trên cùng chuyển đổi giữa các bản dịch có sẵn.
