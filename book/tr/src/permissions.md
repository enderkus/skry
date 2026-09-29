# İzinler ve en az yetki

skry **sıradan, yetkisiz bir kullanıcı** olarak çalışacak şekilde tasarlandı.
Asla `sudo` kullanmaz. Linux'ta verilerin çoğunu herkes okuyabilir; birkaç
kalem bir grup üyeliği ister. Bir şeye izin verilmediğinde skry o kalem için
`n/a (neden)` gösterir ve geri kalan her şey çalışmaya devam eder.

## Ne neye ihtiyaç duyar

| Veri | Gerekli | Olmazsa |
| --- | --- | --- |
| CPU, bellek, yük, çalışma süresi, ağ, disk I/O, disk kullanımı | hiçbir şey (herkesin okuyabildiği `/proc` ve `df`) | — |
| CPU/bellekle süreç listesi | hiçbir şey | `/proc` `hidepid=2` ile bağlandıysa yalnızca kendi süreçleriniz görünür |
| Süreç sahipleri | hiçbir şey | — |
| Dinleyen portlar | hiçbir şey | — |
| IP adresleri, işletim sistemi, çekirdek | hiçbir şey | — |
| Çökmüş systemd birimleri | genellikle hiçbir şey (D-Bus) | Normal kullanıcıların D-Bus erişimi olmayan sistemlerde `n/a (systemctl not permitted)` |
| Başarısız SSH girişleri (journal) | `systemd-journal` ya da `adm` grubu (bazen `wheel`) | `n/a (logs not readable …)` |
| Başarısız SSH girişleri (`/var/log/auth.log`, `secure`) | `adm` grubu (Debian/Ubuntu) ya da root (RHEL'de `secure` yalnızca root'a açık) | `n/a (logs not readable …)` |
| Bekleyen güncellemeler, Debian/Ubuntu/Alpine | hiçbir şey | — |
| Bekleyen güncellemeler, RHEL ailesi | root **ve** `security.rpm_updates = true` | `n/a (requires root on RPM systems)` ya da `n/a (disabled on RPM systems; …)` |
| Konteynerler (Docker) | `/var/run/docker.sock` erişimi (`docker` grubu) | `n/a (docker: permission denied)` |
| Konteynerler (Podman) | kendi rootless konteynerleriniz için hiçbir şey | yalnızca kullanıcınızın konteynerleri gösterilir |

## İzleme hesabı için önerilen kurulum

Debian / Ubuntu:

```sh
sudo useradd -m -s /bin/sh skry
sudo usermod -aG adm,systemd-journal skry
```

RHEL / Rocky / Alma:

```sh
sudo useradd -m -s /bin/sh skry
sudo usermod -aG systemd-journal skry     # journal (başarısız girişler)
```

Sonra izleme açık anahtarınızı `~skry/.ssh/authorized_keys` dosyasına koyun
(bkz. [Kimlik doğrulama](authentication.md)).

> [!WARNING]
> **`docker` grubu hakkında:** `docker` grubuna üyelik, fiilen sunucuda root
> erişimi demektir (gruptaki herkes ayrıcalıklı bir konteyner başlatabilir).
> İzleme hesabını buna yalnızca bunu kabul ediyorsanız ekleyin. Eklemezseniz skry
> konteynerleri basitçe `n/a` gösterir.

## İzleme anahtarını daha da kısıtlamak (isteğe bağlı)

Normal komut çalıştırma mümkün kaldığı sürece anahtarın neler yapabileceğini
`authorized_keys` içinde sınırlayabilirsiniz:

```text
restrict,pty ssh-ed25519 AAAA... skry-izleme
```

`restrict`; port, ajan ve X11 yönlendirmesini kapatır. (skry'nin pty'ye
ihtiyacı yoktur ama `pty` zararsızdır; kaldırabilirsiniz.) `command="…"`
zorunlu komutlarını **kullanmayın**: skry'nin betiğinin yerine geçerler ve
toplama başarısız olur.

## Neden root değil?

Root olarak giriş yapmak skry'ye, dnf güncelleme kontrolleri ve yalnızca
root'un okuyabildiği loglar dışında gerekli hiçbir şey kazandırmaz; üstelik
çalınan bir izleme anahtarının bir root anahtarı olması demektir. Yukarıdaki
gruplarla yetkisiz hesabı tercih edin.

## Sunucu ne görür

Sunucu her skry oturumu için izleme hesabına ait **tek** bir SSH girişi
(skry durduğunda da bir çıkış) kaydeder. Her ölçüm o kullanıcı olarak birkaç
kısa ömürlü süreç (`sh`, `cat`, `grep`, `awk`, `df`, `ps`, …) çalıştırır.
Hiçbir dosya oluşturulmaz.
