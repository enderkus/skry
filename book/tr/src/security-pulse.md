# Güvenlik nabzı

Güvenlik nabzı her sunucu için dört soruyu cevaplar:

1. **Biri içeri girmeye mi çalışıyor?** — son 24 saatteki başarısız SSH girişleri
2. **Yamalı mı?** — bekleyen güvenlik güncellemeleri
3. **Dinlememesi gereken bir şey dinliyor mu?** — izin listesi dışındaki TCP portları
4. **Sertifikalarımın süresi dolmak üzere mi?** — TLS sertifika bitiş tarihleri

Bunu üç yerde görürsünüz: terminal arayüzünün `S` ekranı ve Güvenlik paneli
(`5`), web panelinin Security sekmesi ve `skry security` komutu.

```sh
skry security @production
skry security @production --json
```

```text
HOST    PULSE     FAILED LOGINS 24H         UPDATES               LISTENING         FINDINGS
web-1   ok        3 (top 203.0.113.50 ×3)   0 security / 4 total  as allowed
db-1    critical  1840 (top 198.51.100.23 ×1702)  5 security / 23 total  unexpected: 5432  1840 failed SSH logins in 24h …

TLS ENDPOINT          STATUS    EXPIRY              ISSUER
www.example.com:443   ok        expires in 64 days  R11
api.example.com:443   critical  expires in 5 days   R10
```

## Bulgular ve seviyeleri

| Bulgu | Ne zaman | Seviye |
| --- | --- | --- |
| `failed_logins` | 24 saatte en az `security.failed_login_threshold` (varsayılan 50) başarısız SSH girişi | warning; eşiğin 10 katında (500) **critical** |
| `security_updates` | en az bir bekleyen güvenlik güncellemesi | warning |
| `unexpected_ports` | bir TCP portu loopback olmayan bir adreste dinliyor ve grubun `allowed_ports` listesinde değil | warning |
| TLS sertifikası | geçersiz ya da `tls_critical_days` (7) gün içinde bitiyor | critical |
| TLS sertifikası | `tls_warning_days` (21) gün içinde bitiyor | warning |

Bulgular kutucuklarda `⚑` işareti olarak görünür ve [uyarı](alerts.md)
gönderebilir (kategori `security`). Sunucunun sağlık rengini **değiştirmezler**
— bir sunucu sağlıklıyken de yamaya ihtiyaç duyabilir.

## Başarısız SSH girişleri

skry'nin baktığı yerler, sırasıyla:

1. **systemd journal** (`journalctl`): son 24 saatte `sshd` ve `sshd-session`
   mesajları — systemd çalıştıran her sunucuda kullanılır.
2. Değilse okunabilen ilk dosya: `/var/log/auth.log` (Debian, Ubuntu),
   `/var/log/secure` (RHEL ailesi), `/var/log/messages` (syslog'lu Alpine);
   artı bir önceki döndürülmüş dosya (`.1`).

Sayılanlar: `Failed password`, `Failed publickey`,
`Failed keyboard-interactive` ya da `Invalid user` içeren satırlar. `from`
sonrasındaki adresler de sayılır ve en çok deneyen kaynaklar gösterilir.
(Var olmayan bir kullanıcıya yapılan parola denemesi genellikle iki satır
üretir — bkz. [hesaplamalar](calculations.md#başarısız-ssh-girişleri).)

**İzinler:** başka kullanıcıların journal kayıtlarını ya da auth log'u okumak
için `adm` (Debian/Ubuntu) veya `systemd-journal` grubuna, bazı sistemlerde
`wheel`'e üyelik gerekir. Yoksa skry
`n/a (logs not readable (needs adm or systemd-journal group))` gösterir —
asla sıfır deneme varmış gibi davranmaz. [İzinler](permissions.md) sayfasına
bakın.

## Bekleyen güvenlik güncellemeleri

**Sunucuda zaten bulunan paket önbelleğinden** okunur; skry onu asla yenilemez:

| Dağıtım | Komut | Güvenlik tespiti |
| --- | --- | --- |
| Debian, Ubuntu | `apt-get -s dist-upgrade` (simülasyon) | paket bir `…-security` deposundan geliyor |
| Alpine | `apk version -l '<'` | mümkün değil — güvenlik `?` gösterir |
| RHEL, Rocky, Alma | `dnf -C -q updateinfo list` — **yalnızca** `security.rpm_updates = true` ise ve skry root olarak giriyorsa | duyurunun önem derecesi `/Sec.` ile bitiyor |

dnf neden varsayılan olarak kapalı? dnf, yalnızca önbellekten okurken bile
kendi log dosyalarına ekleme yapar; bu, skry'nin "hiçbir şey yazmaz" sözünü
bozar ve root gerektirir. Bunu kabul ediyorsanız yapılandırmada açın.

Sayılar çok düşük görünüyorsa sunucunun önbelleği eski olabilir.
Debian/Ubuntu'da önbelleği `apt-daily.timer` yeniler; `ls -l /var/lib/apt/lists`
ile kontrol edin.

## Dinleyen portlar ve izin listeleri

skry dinleyen her **TCP** soketini listeler (`ss`, `netstat` ya da
`/proc/net/tcp`'den). UDP dahil değildir.

Beklenmeyen portlar için bulgu almak istiyorsanız gruba bir izin listesi verin:

```toml
[groups.web]
hosts = ["web1", "web2"]
allowed_ports = [22, 80, 443]
```

- Yalnızca **loopback olmayan** bir adreste dinleyen portlar sayılır.
  `127.0.0.1:5432`'de dinleyen bir veritabanı dışarıya açık değildir ve
  sorun yoktur; `0.0.0.0:5432`'de dinleyen işaretlenir.
- `allowed_ports` olmayan grupların port politikası yoktur: portlar
  listelenir ("no policy") ama asla işaretlenmez.
- Birden çok gruptaki bir sunucu, gruplarının izinli portlarının birleşimini
  dinleyebilir.

## TLS sertifika bitiş tarihleri

HTTPS (ya da herhangi bir TLS) uç noktalarını genel olarak ya da grup başına
listeleyin:

```toml
[security]
tls = ["www.example.com:443", "mail.example.com:993"]

[groups.web]
tls = ["api.example.com"]        # port verilmezse 443
```

Kontrol sunuculardan değil, **sizin bilgisayarınızdan** yapılır: skry gerçek
bir TLS bağlantısı kurar, sertifikayı okur ve zinciri skry'nin içine gömülü
Mozilla kök sertifikalarına göre doğrular (işletim sisteminizin deposuna göre
değil). Kalan gün sayısını, konuyu, yayıncıyı ve varsa doğrulama sorununu
(süresi dolmuş, yanlış isim, bilinmeyen yayıncı…) bildirir.

- Terminal arayüzünde ve `serve`'de uç noktalar başlangıçta ve sonra saatte bir
  kontrol edilir. `skry security` ve `skry snapshot` her çalıştırmada kontrol
  eder.
- Her kontrolün 10 saniyelik bir zaman aşımı vardır.

> [!NOTE]
> **Özel/kurum içi bir CA**'nın verdiği sertifikalar, o CA genel kök
> sertifikalar arasında olmadığı için geçersiz olarak bildirilir. Bitiş tarihi
> yine doğru gösterilir.
