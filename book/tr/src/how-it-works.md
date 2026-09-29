# Büyük resim

Bu sayfa, `skry web1` yazmanızla ekranda sayıları görmeniz arasında neler
olduğunu teknik jargona boğmadan anlatır.

```text
 SİZİN BİLGİSAYARINIZ                                 SUNUCUNUZ (web1)
 ┌──────────────────────────────┐                     ┌───────────────────────────┐
 │ skry                         │   tek SSH oturumu   │ sshd                      │
 │  1. ~/.ssh/config'i okur     │ ══════════════════▶ │  (normal SSH sunucusu)    │
 │  2. host anahtarını denetler │                     │                           │
 │  3. anahtarınızla girer      │   her 2 saniyede:   │  sh -c '...betik...'      │
 │  4. küçük bir betik yollar   │ ──────────────────▶ │   cat /proc/stat          │
 │                              │                     │   cat /proc/meminfo       │
 │  6. cevabı ayrıştırır        │ ◀────────────────── │   df -P ...               │
 │  7. hız ve sağlığı hesaplar  │   düz metin cevap   │  (biter, iz bırakmaz)     │
 │  8. ekranı çizer             │                     │                           │
 │  9. özeti yerelde saklar     │                     └───────────────────────────┘
 └──────────────────────────────┘
```

## Adım adım

1. **İsmi çözümle.** `web1`, gerçek adresi, kullanıcıyı, portu, anahtar
   dosyalarını ve varsa atlama sunucularını bulmak için `~/.ssh/config`
   içinde aranır — tıpkı `ssh` komutunun yapacağı gibi.
2. **Sunucuyu doğrula.** Sunucu host anahtarını sunar. skry onu
   `known_hosts` dosyanızla karşılaştırır. Bilinmeyen ya da değişmiş anahtarlar
   bağlantıyı burada durdurur ([Host anahtarları](host-keys.md)).
3. **Giriş yap.** skry anahtarlarınızı sunar: önce SSH ajanınızdakileri, sonra
   anahtar dosyalarını ([Kimlik doğrulama](authentication.md)).
4. **Bağlantıyı açık tut.** Bu tek SSH bağlantısı her ölçümde yeniden
   kullanılır; pahalı el sıkışma yalnızca bir kez olur.
5. **Her ölçümde** (varsayılan olarak her 2 saniyede) skry bu bağlantının
   içinde hafif bir kanal açar ve **tek** bir kabuk komutu çalıştırır. Bu komut
   kısa bir POSIX `sh` betiğidir: `/proc` altındaki birkaç dosyanın içeriğini ve
   birkaç standart aracın çıktısını, aralarında işaret satırları olacak
   şekilde yazdırır. Betik biter, kanal kapanır. Sunucuda hiçbir şey çalışmaya
   devam etmez.
6. **Ayrıştır.** skry metni işaretlerden böler ve her parçayı kendine ait bir
   ayrıştırıcıyla okur.
7. **Hesapla.** Linux'taki birçok sayaç yalnızca artar ("açılıştan beri
   harcanan CPU zamanı", "açılıştan beri alınan bayt"). skry önceki ölçümü
   şimdikinden çıkarır ve geçen süreye bölerek hızları ve yüzdeleri bulur.
   Sonra değerleri eşiklerinizle ve sunucunun kendi geçmişiyle (baseline'lar)
   karşılaştırır.
8. **Göster.** Terminal arayüzü, web paneli ve `/metrics` aynı güncel durumu
   okur.
9. **Hatırla.** Her ölçümün kısa bir özeti **sizin** bilgisayarınızdaki yerel
   bir SQLite veritabanına yazılır; böylece zamanda geriye gidebilirsiniz.

## Neden çok sayıda komut yerine tek betik?

On ayrı komut, ağda on gidiş-dönüş ve sshd'nin başlattığı on süreç demek. Tek
betik, ölçüm başına **tek** gidiş-dönüş demektir; bu hem sunucudaki hem ağdaki
yükü çok küçük tutar hem de bir ölçümdeki tüm değerlerin aynı andan gelmesini
sağlar.

## Neden düz SSH?

- Her Linux sunucusunda zaten var.
- Zaten güvenli, denetlenmiş ve güvenlik duvarlarınızdan geçmesine izin
  verilmiş durumda.
- Kim olduğunuzu (anahtarlarınız) ve sunucunun kim olduğunu (host anahtarları)
  zaten biliyor.
- Yeni bir şey açmaya, kurmaya, güncellemeye ya da güvenmeye gerek yok.

## skry'nin sunucuda **yapmadıkları**

- Sunucuya hiçbir dosya kopyalamaz.
- Geçici dosya oluşturmaz.
- `sudo` kullanmaz, root istemez.
- Çalışmaya devam eden hiçbir şey başlatmaz.
- Durum değiştiren paket yöneticisi komutları çalıştırmaz (`apt update`,
  `dnf makecache`, …).

Dürüst bir not: her SSH girişi gibi **sizin girişiniz de sunucu tarafından
kaydedilir** (örneğin `/var/log/auth.log` ya da journal içine). Bu sshd'nin
normal işidir, skry'nin bir şey yazması değil.

Betiğin tam hâlini istediğiniz zaman yazdırabilirsiniz — hiçbir şey
çalıştırılmaz:

```sh
skry script          # her ölçümde çalışan kısım
skry script --all    # daha seyrek çalışan yavaş bölümler dahil
```

Sonraki sayfalar daha derine iner: [her ölçüm](tick.md),
[betik](remote-script.md), [hesaplamalar](calculations.md).
