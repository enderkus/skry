# Sözlük

**Ajan (izleme)** — veri toplamak için her sunucuya kurulan program. skry
*ajansızdır*: hiç yoktur.

**Ajan (SSH)** — bilgisayarınızda kilidi açılmış SSH anahtarlarını tutan ve
girişleri imzalayan program (`ssh-agent`, Pageant). Farklı bir şey, aynı kelime.

**Anlık görüntü (snapshot)** — filonun bir andaki Markdown + JSON kaydı.

**Aşım (breach)** — uyarı ya da kritik eşiğine ulaşmış ya da onu geçmiş bir
metrik.

**Baseline** — skry'nin bir sunucu ve metrik için normal olarak öğrendiği değer.
→ [Baseline'lar](baselines.md)

**Bastion / atlama sunucusu** — diğerlerine ulaşmak için içinden geçmeniz
gereken sunucu. → [Bastion](proxyjump.md)

**Bekleme süresi (cooldown)** — aynı uyarı için iki bildirim arasındaki en
kısa süre.

**Bulgu (finding)** — bir güvenlik sorunu: çok sayıda başarısız giriş, bekleyen
güvenlik güncellemeleri, beklenmeyen portlar, bir TLS sorunu.

**Çekirdek başına yük (load per core)** — 1 dakikalık yük ortalamasının
çekirdek sayısına bölümü.

**Eşik (threshold)** — CPU, bellek, disk ya da çekirdek başına yük için bir
uyarı ya da kritik sınırı.

**EWMA** — üstel ağırlıklı hareketli ortalama: yakın değerlerin daha çok
sayıldığı bir ortalama. Baseline'larda kullanılır.

**Filo (fleet)** — izlediğiniz tüm sunucular.

**Host anahtarı** — sunucunun kimliğini kanıtladığı anahtar.
→ [Host anahtarları](host-keys.md)

**İşaret (marker)** — uzak betiğin çıktısındaki bölümleri ayıran, rastgele
değer içeren satır.

**İzin listesi (allowlist)** — bir grubun `allowed_ports` değeri: dinlemesine
izin verilen portlar.

**`known_hosts`** — güvendiğiniz host anahtarlarını listeleyen dosya.

**Kutucuk (tile)** — filo ekranında bir sunucunun kutusu.

**Loopback** — yalnızca aynı makineye ulaşan adresler (`127.0.0.1`, `::1`).

**`n/a`** — kullanılamıyor: eksik araç, eksik izin ya da geçerli değil. Asla
bir sorun olarak değerlendirilmez.

**Nonce** — işaretlerdeki, her ölçümde yenilenen rastgele değer.

**Ölçüm (tick)** — bir sunucu için bir ölçme turu (varsayılan 2 sn'de bir).

**Parmak izi (fingerprint)** — bir host anahtarının kısa hâli, ör.
`SHA256:Zm9v…`; anahtarları gözle karşılaştırmak için kullanılır.

**POSIX sh** — Unix benzeri her sistemin desteklediği standart kabuk dili;
skry'nin betiği yalnızca bunu kullanır.

**Probe** — bazı sunucularda kullanılamayabilecek bir veri kalemi (portlar,
konteynerler, birimler, girişler, güncellemeler).

**`/proc`** — Linux çekirdeğinin canlı sistem bilgilerini metin dosyaları
olarak yayımladığı sanal dizin.

**ProxyJump** — bir bastion üzerinden bağlanmayı sağlayan SSH ayarı.

**RSS** — resident set size: bir sürecin RAM'de gerçekten kapladığı bellek.

**Sapma (deviation)** — z-skoru eşiği aşan bir değer; camgöbeği gösterilir.

**Sayaç (counter)** — yalnızca büyüyen bir sayı (CPU tikleri, alınan bayt);
hızlar iki sayaç arasındaki farktan hesaplanır.

**SSE (server-sent events)** — panelin canlı güncellemeleri alma yöntemi.

**Steal** — bir sanal makinenin istediği ama hipervizörün başkasına verdiği CPU
zamanı.

**Yavaş bölümler** — her ölçüm yerine 60 sn'de bir toplanan veriler.

**Yük ortalaması (load average)** — 1, 5 ve 15 dakika boyunca çalışan ya da
çalışmayı bekleyen süreçlerin ortalama sayısı.

**z-skoru** — bir değerin olağan değerin kaç "olağan değişkenlik" üstünde
olduğu.
