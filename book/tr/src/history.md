# Geçmiş ve zamanda yolculuk

Sunucuları terminal arayüzüyle (`skry <sunucular>`) ya da web paneliyle
(`skry serve`) izlediğinizde skry gördüklerini **sizin** bilgisayarınızdaki
küçük bir veritabanına kaydeder. Böylece geriye giderek filonun ya da tek bir
sunucunun daha önce nasıl göründüğünü görebilirsiniz.

## Her zaman açık mı?

- Terminal arayüzü ve `skry serve` için **varsayılan olarak açık**.
- Tek seferlik komutlar (`--once`, `find`, `security`, `snapshot`) için
  **asla** — veritabanına dokunmazlar.
- Tek bir çalıştırma için `--no-history` ile, kalıcı olarak da şöyle kapatın:

  ```toml
  [history]
  enabled = false
  ```

Geçmiş kapalıyken her şey yine çalışır; yalnızca zamanda geriye gidemezsiniz,
geçmiş grafikleri boş kalır ve [baseline'lar](baselines.md) skry her
başladığında sıfırdan başlar.

## Neler saklanır

| Ne | Ne sıklıkla | İçerik |
| --- | --- | --- |
| Özet | her ölçümde, sunucu başına | durum, CPU %, bellek %, en dolu disk %, çekirdek başına yük, ağ giriş/çıkış |
| Tam kayıt | sunucu başına 30 sn'de bir (`history.detail_interval`) | sunucu ayrıntılarında gördüğünüz her şey: süreçler, diskler, arayüzler, konteynerler, birimler, bulgular… |
| Baseline'lar | 5 dakikada bir ve skry kapanırken | sunucu ve metrik başına öğrenilmiş "normal" değerler |

Erişilemeyen dönemler de kaydedilir; bir sunucunun *ne zaman* gittiğini
görebilirsiniz.

## Ne kadar saklanır, nasıl küçülür

Yakın geçmiş ayrıntılıdır; eski veriler dosya küçük kalsın diye kendiliğinden
seyreltilir:

```text
 şimdi ◀──── son 1 saat ────▶◀──── 1 sa … 6 sa ────▶◀──── 6 sa … 24 sa ────▶ silinir
         her ölçüm              1 dakikalık ortalama     5 dakikalık ortalama
         tam kayıt / 30 sn      tam kayıt / 5 dk         tam kayıt / 5 dk
```

- Ortalamalar kendi aralıklarındaki **en kötü** durumu korur; 20 saniyelik bir
  kesinti bir saat sonra da "unreachable" olarak görünür.
- `history.retention_hours`'tan (varsayılan 24) eski her şey silinir.
- Bu bakım, skry çalışırken beş dakikada bir yapılır.

## Terminal arayüzünde geriye gitmek

| Tuş | İşlem |
| --- | --- |
| `[` / `]` | Bir dakika geri / ileri |
| `{` / `}` | 15 dakika geri / ileri |
| `L` ya da `End` | Canlıya dön |

Geçmişteyken:

- Üst satır sarı renkle `HISTORY 2026-09-29 11:13:00 (-47m 0s)` gösterir ve
  zaman çizelgesindeki nokta sola kayar.
- **Filo kutucukları** o an için saklanan özeti gösterir: durum, CPU, bellek,
  en dolu disk, **çekirdek başına** yük ve ağ.
- **Sunucu ayrıntıları**, o an ya da öncesindeki en yakın tam kaydı gösterir;
  zaman çizelgesi satırında kaydın tam zamanı `details recorded 11:12:41`
  şeklinde yazar.
- O anda verisi olmayan bir sunucu (henüz izlenmiyordu ya da skry çalışmıyordu)
  pending olarak görünür.
- En eski kayıttan daha geriye gidemezsiniz; *şimdi*yi geçecek kadar ileri
  gitmek canlıya döndürür.
- `s`, **baktığınız anın** anlık görüntüsünü alır; böylece bir olayın geçmişteki
  durumunu belgeleyebilirsiniz.

Siz geçmişe bakarken canlı veri arka planda toplanmaya devam eder.

## Dosya nerede

| Platform | Varsayılan yol |
| --- | --- |
| Linux | `~/.local/share/skry/history.db` |
| macOS | `~/Library/Application Support/skry/history.db` |
| Windows | `%APPDATA%\skry\data\history.db` |

`history.path` ile değiştirebilirsiniz. Dosya normal bir SQLite
veritabanıdır; merak ederseniz `sqlite3` ile açabilirsiniz (`samples`,
`details`, `baselines` tabloları). skry çalışmıyorken silmek güvenlidir.

## Ne kadar büyür?

Bir özet satırı birkaç düzine bayttır; bir tam kayıt birkaç kilobayttır.
Varsayılanlarla, süreç ve konteyner sayısına bağlı olarak 24 saat için sunucu
başına kabaca **1–3 MB** bekleyin.
