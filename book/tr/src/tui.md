# Terminal arayüzü, ekran ekran

`skry <sunucular>` ile başlatın. Tuş listesi için istediğiniz an `?`'e,
çıkmak için `q`'ya basın. Arayüzün üç ekranı vardır: **Filo**, **Sunucu
ayrıntıları** ve **Güvenlik nabzı**. (Arayüzdeki etiketler İngilizcedir; bu
sayfa her birinin ne anlama geldiğini açıklar.)

## Her ekranın çerçevesi

```text
 skry  8 hosts ● 3 ok ● 1 deviation ● 1 warning ● 1 critical ● 2 unreachable  sort:status    LIVE  every 2s
 …
 09:00 ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━● now
 ←↑↓→ move  Enter details  / filter  f status  o sort  [ ] time  S security  s snapshot  ? help  q quit
```

- **Üst satır**: sunucu sayısı, her durumda kaç sunucu olduğu, geçerli
  sıralama, etkin filtreler ve sağda ya `LIVE every 2s` (canlı, 2 saniyede bir)
  ya da geçmişe bakıyorsanız `HISTORY 2026-09-29 11:13:00 (-47m 0s)`.
- **Zaman çizelgesi** (sondan ikinci satır): solda kaydedilen en eski andan
  sağda *şimdi*ye (`now`) kadar. Nokta nerede olduğunuzu gösterir.
  [Geçmiş ve zamanda yolculuk](history.md) sayfasına bakın.
- **Alt satır**: bu ekranda çalışan tuşlar. Mesajlar ("snapshot saved to …"
  gibi) da burada görünür.

## Filo ekranı

Her sunucu için bir kutucuk; terminalinize sığdığı kadar sütun (her kutucuk
30 karakter genişliğinde).

```text
┏ db-1 ━━━━━━━━━━━━━ critical ┓
┃CPU ████████████████▉·  94%  ┃
┃MEM ███████████████▉··  88%  ┃
┃DSK ████████████▊·····  71%  ┃
┃LD 26.69 ↓1.1MiB/s ↑317KiB/s ┃
┃✖ 1 unit ⚑ 3 security        ┃
┗━━━━━━━━━━━━━━━ @production ━┛
```

| Öğe | Anlamı |
| --- | --- |
| Çerçeve rengi ve sağ üstteki etiket | Durum (yeşil ok, camgöbeği deviation, sarı warning, kırmızı critical, eflatun unreachable, gri pending) |
| Kalın çerçeve, vurgulu isim | Seçili kutucuk |
| `CPU`, `MEM`, `DSK` çubukları | Yüzde; eşiklerinize göre yeşil / sarı / kırmızı; `·` boş kısmı gösterir |
| `LD` | 1 dakikalık yük ortalaması (rengi çekirdek başına yüke göre) |
| `↓` `↑` | Saniyede alınan / gönderilen ağ trafiği (birincil arayüzler) |
| `◆ memory z5.9` | Bu değer bu sunucu için olağandışı ([baseline'lar](baselines.md)) |
| `✖ 1 unit` | Çökmüş systemd birimi sayısı |
| `⚑ 3 security` | Güvenlik bulgusu sayısı (renk = en kötü seviye) |
| `@production` | Sunucunun ait olduğu (ilk) grup |

Hata veren bir sunucu çubuklar yerine nedeni gösterir:

```text
╭ staging-1 ──── unreachable ╮
│✖ host key                  │
│unknown host key for        │
│staging-1.example.com …     │
```

Ekrana sığmayacak kadar sunucu varsa ızgara seçimle birlikte kayar ve köşede
`rows 1-4 of 9` (9 satırın 1-4'ü) gösterir.

**Tuşlar**

| Tuş | İşlem |
| --- | --- |
| oklar ya da `h` `j` `k` `l` | Seçimi taşı |
| `g` / `G` ya da `Home` | İlk / son sunucu |
| `Enter` | Sunucu ayrıntılarını aç |
| `/` | Filtre yaz (sunucu adı, grup ya da sunucunun kendi hostname'i ile eşleşir); `Enter` uygular, `Esc` temizler |
| `f` | Durum filtresi: hepsi → sorunlular (ok olmayan her şey) → yalnızca erişilemeyenler |
| `o` | Sıralama: durum (en kötüsü önce) → isim → CPU → bellek → disk (en yükseği önce) |
| `Esc` | Filtreyi ve durum filtresini temizle |
| `S` | Güvenlik nabzı ekranı |
| `s` | **Şu an görünen tüm sunucuların** anlık görüntüsü (filtrenize uyar) |

## Sunucu ayrıntıları ekranı

```text
 db-1  critical  deploy@10.0.1.12:22
 Rocky Linux 9.4 (Blue Onyx) · 6.1.0-25-amd64 x86_64 · up 23d 2h · 8 cores · AMD EPYC 7B13
 1 Overview │ 2 Processes │ 3 Network & Disks │ 4 Services & Containers │ 5 Security
```

Başlık; isim, durum, adres, işletim sistemi, çekirdek, mimari, çalışma süresi,
çekirdek sayısı ve CPU modelini gösterir. Sunucu hata veriyorsa panellerin
üstünde hatayı ve ne zamandan beri sürdüğünü gösteren bir kutu çıkar (bilinen
son veriler altında gösterilmeye devam eder).

| Tuş | İşlem |
| --- | --- |
| `Tab` / `→` | Sonraki panel |
| `Shift-Tab` / `←` | Önceki panel |
| `1`–`5` | Doğrudan bir panele git |
| `n` / `p` | Sonraki / önceki sunucu (filonun geçerli sırasına göre) |
| `↑` `↓` `PgUp` `PgDn` | Kaydır (süreçler, güvenlik) |
| `Esc` / `Backspace` | Filoya dön |
| `s` | **Yalnızca bu sunucunun** anlık görüntüsü |

### 1 Overview (Genel bakış)

- **CPU** (user / system / iowait / steal dağılımıyla), **MEM** (kullanılan /
  toplam), **SWAP**, **DISK** (en dolu dosya sistemi ve bağlama noktası)
  çubukları.
- **LOAD** 1/5/15 dakika, çekirdek başına yük, görev sayısı ve çalışan görevler.
- **NET** toplam alma / gönderme hızı.
- **Per core**: her CPU çekirdeği için küçük bir çubuk.
- **Sorunlar**: `▲` eşik aşımları, `◆` baseline sapmaları, `⚑` güvenlik
  bulguları — her biri bir cümle olarak, örneğin
  `▲ disk 91.0% ≥ 90.0% (critical)`.
- Sağ taraf: **CPU history** ve **Memory history** grafikleri (yerel geçmişten
  son 30 dakika), sunucu adı, adresler, süreç sayısı, çökmüş birimler,
  konteynerler ve verinin ne zaman ölçüldüğü.

### 2 Processes (Süreçler)

CPU'ya göre en meşgul süreçler ile belleğe göre en büyükler: PID, kullanıcı,
CPU% (**tek** çekirdeğe göre, 100'ü geçebilir), bellek %, RSS, durum (`R`
çalışıyor, `S` uyuyor, `D` disk bekliyor, `Z` zombi), komut adı.

### 3 Network & Disks (Ağ ve diskler)

- **network**: arayüz başına alma/gönderme hızı, açılıştan beri toplamlar,
  hatalar/düşürülenler.
- **disk I/O**: bütün disk başına okuma/yazma hızı, IOPS, doluluk oranı (util).
- **filesystems**: bağlama noktası, aygıt, kullanılan, boyut, kullanım çubuğu.

### 4 Services & Containers (Servisler ve konteynerler)

- **systemd**: durumları ve açıklamalarıyla çökmüş birimler, "no failed
  systemd units" (çökmüş birim yok) ya da neden kullanılamadığı (systemd
  sunucusu değil ya da izin yok).
- **containers**: Docker ya da Podman için ad, imaj, durum, CPU ve bellek;
  durdurulmuş konteynerler soluk gösterilir.

### 5 Security (Güvenlik)

- Adresleriyle **dinleyen TCP portları**; grubun izin listesi dışındaki
  portlar kırmızı ve "not in allowlist" notuyla, yalnızca loopback'te dinleyen
  portlar soluk gösterilir.
- **Başarısız SSH girişleri (24 saat)**: toplam, kaynak, en çok deneyen
  adresler.
- **Bekleyen güncellemeler**: toplam, güvenlik sayısı, paket adları.

## Güvenlik nabzı ekranı

Filodayken `S`'ye basın. Her sunucu için bir satır:

| Sütun | Anlamı |
| --- | --- |
| HOST | Sunucu adı |
| PULSE | En kötü bulgu seviyesi (ok / warning / critical) |
| LOGINS | 24 saatteki başarısız SSH girişleri |
| SEC/UPD | Bekleyen güvenlik güncellemeleri / tüm bekleyen güncellemeler |
| UNEXPECTED PORTS | İzin listesi dışındaki portlar, `ok` ya da "(no policy)" (politika yok) notuyla dinleyen portlar |
| FINDINGS | Her bulgu bir cümle olarak |

Altında, yapılandırılmışsa bir **TLS sertifikaları** tablosu: uç nokta, durum,
bitiş, konu (subject), yayıncı (issuer). Dönmek için `S` ya da `Esc`.
[Güvenlik nabzı](security-pulse.md) sayfasına bakın.

## Yardım penceresi

`?` ya da `F1` tüm tuşları gösterir. Herhangi bir tuş kapatır.

## Terminal ipuçları

- Unicode ve 256 renk destekleyen bir terminal kullanın (her modern terminal
  olur; Windows'ta Windows Terminal kullanın).
- Arayüz pencere boyutuna uyum sağlar; daha fazla sütun için pencereyi
  genişletin.
- Arayüz ekrana asla log mesajı yazmaz. Arka planda neler olduğunu görmek için
  `--log-file skry.log` (ve `-v` ya da `-vv`) kullanın.
