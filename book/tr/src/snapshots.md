# Olay anlık görüntüleri

Bir olay sırasında *her şeyin şu an nasıl göründüğünün* kaydını istersiniz:
olay sonrası değerlendirme için, bir kayıt (ticket) için ya da işi bir
arkadaşınıza devretmek için. Bir anlık görüntü (snapshot), seçili sunucuların
eksiksiz durumunu iki dosyaya kaydeder:

- insanların okuyabileceği bir **Markdown** raporu (bir kayda, wiki'ye,
  sohbete yapıştırın) ve
- araçlar için her değeri içeren bir **JSON** dosyası.

## Anlık görüntü almak

**Terminal arayüzünde:** `s`'ye basın.

- Filo ekranında: **görünen** tüm sunucular (filtreniz geçerlidir — önce
  etkilenen sunuculara filtre uygulayıp sonra `s`'ye basın).
- Bir sunucunun ayrıntılarında: yalnızca **o sunucu**.
- [Geçmiş modunda](history.md): baktığınız andaki durum.

Alt satır onaylar: `snapshot of 3 host(s) saved to ./skry-snapshot-20260929-120000.md`.

**Komut satırından:**

```sh
skry snapshot @production
skry snapshot web1 db1 --out ~/olaylar/2026-09-29
skry snapshot @production --post          # webhook'lara da gönder
skry snapshot @production --json          # JSON'u ayrıca stdout'a yaz
```

Komut, bir saniye arayla iki ölçüm alır (CPU ve ağ hızları dahil olsun diye),
yapılandırılmış TLS uç noktalarını kontrol eder, dosyaları yazar ve yollarını
yazdırır.

## Dosyalar nereye gider

1. Verilmişse `--out DIZIN`,
2. değilse yapılandırmadaki `snapshot.dir`,
3. o da yoksa geçerli dizin.

Dosya adları `skry-snapshot-YYYYMMDD-HHMMSS.md` ve `.json` biçimindedir (UTC
saati).

## Raporda neler var

- Bir özet tablosu: her sunucu için durum, CPU, bellek, disk, çekirdek başına
  yük, çalışma süresi ve notlar (hatalar, eşik aşımları, sapmalar).
- TLS sertifika tablosu (yapılandırıldıysa).
- Sunucu başına: adres, gruplar, işletim sistemi, çekirdek, CPU modeli, çalışma
  süresi, IP adresleri; sorunlar (eşik aşımları, baseline sapmaları, güvenlik
  bulguları); CPU dağılımı ve çekirdek başına değerler; yük; bellek ve swap;
  dosya sistemleri; disk I/O; ağ arayüzleri; en yoğun süreçler; çökmüş
  birimler; konteynerler; dinleyen portlar; başarısız SSH girişleri; bekleyen
  güncellemeler.

## Sohbete ya da bir webhook'a göndermek

Anlık görüntü alması gereken webhook'ları işaretleyin:

```toml
[[webhooks]]
kind = "slack"
url = "https://hooks.slack.com/services/<webhook-yolunuz>"
snapshot = true
```

- Slack ve Discord **kısa bir özet** alır (her sunucu için durumu ve ana
  sayılarıyla bir satır, artı eşik aşımları).
- Genel webhook'lar **eksiksiz JSON**'u alır:
  `{"source":"skry","kind":"snapshot","snapshot":{…}}`.

Terminal arayüzünde `s`'ye basmak, `snapshot = true` olan her webhook'a
otomatik olarak gönderir. Komut satırında `--post` ekleyin.
