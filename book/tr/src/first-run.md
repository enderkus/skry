# Adım adım ilk sunucunuz

Bu anlatım sizi sıfırdan tek bir sunucuyu izlemeye kadar götürür. skry'nin
kurulu olduğunu ([Kurulum](installation.md)) ve `ssh sunucunuz` komutunun
parola sormadan giriş yaptığını ([SSH temelleri](ssh-basics.md)) varsayar.

## 1. skry'yi başlatın

`ssh` ile kullandığınız ismin aynısını kullanın:

```sh
skry web1
```

ya da açık bir adres verin:

```sh
skry deploy@203.0.113.10
skry deploy@203.0.113.10:2222     # standart olmayan port
```

## 2. "unknown host key" görürseniz

skry (ve sizin `ssh`'iniz) bu sunucunun anahtarını hiç görmediyse kutucuk mora
döner ve şunu gösterir:

```text
✖ host key
unknown host key for 203.0.113.10 (SHA256:...); verify it,
then rerun with --accept-new
```

Bu, skry'nin sizi koruması. `q` ile çıkın; sonra ya bir kez `ssh` ile bağlanıp
"yes" deyin ya da skry'ye anahtarı kaydetmesini söyleyin:

```sh
skry --accept-new web1
```

`--accept-new` yalnızca `known_hosts` içinde **henüz olmayan** sunucuların
anahtarlarını kabul eder. Kayıtlı olandan farklı bir anahtarı asla kabul
etmez.

## 3. Kutucuğu okuyun

Bir iki saniye sonra kutucuk dolar:

```text
╭ web1 ──────────────────── ok ╮
│CPU ████▋·············  25%    │
│MEM ███████▍··········  41%    │
│DSK █████████▉········  55%    │
│LD 1.52 ↓1.3MiB/s ↑489KiB/s    │
│                               │
╰───────────────── @production ╯
```

| Parça | Anlamı |
| --- | --- |
| `web1` | Yazdığınız isim |
| `ok` (sağ üst) ve çerçeve rengi | Genel sağlık: ok, deviation (sapma), warning (uyarı), critical (kritik), unreachable (erişilemiyor) |
| `CPU` | İşlemcilerin ne kadar meşgul olduğu, tüm çekirdeklere göre yüzde |
| `MEM` | Gerçekten kullanımda olan bellek (Linux'un boşaltabileceği önbellekler **sayılmaz**) |
| `DSK` | En dolu gerçek dosya sistemi, yüzde |
| `LD 1.52` | 1 dakikalık yük ortalaması |
| `↓ ↑` | Saniyede alınan / gönderilen ağ trafiği |
| Beşinci satır | Ek işaretler: `◆` olağandışı değer, `✖ 1 unit` çökmüş servis, `⚑ 2 security` güvenlik bulgusu |
| `@production` | Sunucunun geldiği grup (varsa) |

> [!NOTE]
> CPU ve ağ ilk saniye `n/a` gösterir. Bunlar iki ölçüm arasındaki **farktan**
> hesaplanır; skry'nin göstermeden önce ikinci bir ölçüme ihtiyacı vardır. İkinci
> ölçümü bir saniye sonra alır.

## 4. İçeriye bakın

`Enter`'a basın. Beş panelli host ayrıntı görünümündesiniz. Paneller arasında
`Tab` ya da `1`–`5` rakam tuşlarıyla geçin:

1. **Overview (Genel bakış)** — CPU (çekirdek başına da), bellek, swap, disk,
   yük, ağ, geçmiş grafikleri ve varsa sorunların listesi.
2. **Processes (Süreçler)** — CPU'ya ve belleğe göre en yoğun süreçler.
3. **Network & Disks (Ağ ve diskler)** — arayüz başına trafik, disk başına
   I/O, dosya sistemi başına kullanım.
4. **Services & Containers (Servisler ve konteynerler)** — çökmüş systemd
   birimleri, Docker/Podman konteynerleri.
5. **Security (Güvenlik)** — dinleyen portlar, başarısız SSH girişleri,
   bekleyen güncellemeler.

Filoya dönmek için `Esc`'ye basın.

## 5. Çıkın

`q`'ya basın. skry SSH bağlantısını kapatır. Sunucuda hiçbir şey kalmaz.

## Sadece bir kere mi bakmak istiyorsunuz?

Arayüz olmadan sayıları bir kez görmek için:

```sh
skry web1 --once
```

```text
HOST  STATUS  CPU   MEM    DISK   LOAD  UPTIME  DETAILS
web1  ok      3.1%  41.2%  55.0%  0.12  23d 4h  Debian GNU/Linux 12 (bookworm)
```

Makinece okunabilir çıktı için `--json` ekleyin ([Komut satırı referansı](cli.md)).

## Sonraki adımlar

- Birden çok sunucuyu izleyin: [Çok sayıda sunucu ve gruplar](many-hosts.md)
- skry'nin sunucunuzda az önce ne yaptığını anlayın:
  [Büyük resim](how-it-works.md)
