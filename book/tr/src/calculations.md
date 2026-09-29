# Her sayı nasıl hesaplanır

Bu sayfa skry'nin gösterdiği her sayıyı örneklerle açıklar. skry'deki bir değer
başka bir araçtakinden farklı görünüyorsa, cevap neredeyse her zaman buradadır.

## Sayaçlar ve hızlar: temel fikir

Linux size "CPU şu an %40 meşgul" demez. "Açılıştan beri CPU'lar 1 234 567 tik
çalıştı, 9 876 543 tik boşta kaldı" der. Bu sayılar yalnızca büyür. "Şu an ne
kadar meşgul?" sorusunun cevabı için iki kez ölçer ve **farka** bakarsınız:

```text
                 çalışma tikleri   boşta tikleri
ölçüm 1 (t=0 sn)      1 000            9 000
ölçüm 2 (t=2 sn)      1 080            9 120
fark                     80              120     → meşgul = 80 / (80 + 120) = %40
```

Aynı fikir ağ hızını (şimdi alınan bayt − önce alınan bayt, bölü saniye) ve
disk hızını verir.

**İki ölçüm arasındaki süre** sizin bilgisayarınızın saatinden değil,
sunucunun kendi `/proc/uptime` değerinden alınır. Böylece ağ gecikmeleri
hızları bozmaz. (Uptime yoksa skry yerel saatine döner.)

**Sayaç sıfırlanması.** Bir sayaç *azalırsa* (sunucu yeniden başladı ya da bir
arayüz yeniden oluşturuldu), skry o ölçüm için anlamsız bir değer yerine 0
gösterir.

## CPU

`/proc/stat` içindeki `cpu` satırlarından (tüm değerler tik cinsinden):

| skry'nin gösterdiği | Formül (iki ölçüm arasındaki farklar) |
| --- | --- |
| **CPU %** (kutucuk, genel bakış) | (toplam − idle − iowait) ÷ toplam |
| user (`us`) | (user + nice) ÷ toplam |
| system (`sy`) | (system + irq + softirq) ÷ toplam |
| iowait (`io`) | iowait ÷ toplam |
| steal (`st`) | steal ÷ toplam |
| çekirdek başına | her `cpuN` satırında aynı formül |

Burada *toplam* = user + nice + system + idle + iowait + irq + softirq + steal.

- **CPU %, tüm çekirdeklere göredir.** %100, her çekirdeğin tamamen meşgul
  olduğu anlamına gelir. Tek iş parçacıklı meşgul bir süreç çalıştıran 8
  çekirdekli bir sunucu yaklaşık %12,5 gösterir.
- **iowait, başlık değerinde boşta sayılır**, çünkü CPU beklerken başka işleri
  çalıştırmakta serbesttir. Disk darboğazlarını görebilmeniz için ayrıca
  gösterilir.
- **steal**, sanal makinenizin çalışmak istediği ama hipervizörün CPU'yu başka
  birine verdiği süredir. Sürekli yüksek steal, gürültülü ya da aşırı yüklenmiş
  bir fiziksel sunucu demektir.

## Bellek

`/proc/meminfo` içinden:

| skry'nin gösterdiği | Formül |
| --- | --- |
| **kullanılan** | MemTotal − MemAvailable |
| **MEM %** | kullanılan ÷ MemTotal |
| önbellek (cache) | Buffers + Cached + SReclaimable |
| kullanılan swap | SwapTotal − SwapFree |
| swap % | kullanılan swap ÷ SwapTotal |

`MemAvailable`, çekirdeğin *swap yapmadan* programlara ne kadar bellek
verilebileceğine dair kendi tahminidir; boşaltılabilecek önbellekleri içerir.
Yani skry'nin "kullanılan" değeri **gerçekten dolu olan bellektir**. skry'nin,
önbelleği kullanılmış sayan araçlardan çoğu zaman daha düşük bir sayı
göstermesinin nedeni budur: Linux boş belleği bilerek disk önbelleğiyle
doldurur ve bu sağlıklıdır.

Çok eski çekirdeklerde (3.14 öncesi) `MemAvailable` yoktur; skry o zaman
yaklaşık değer olarak MemFree + Buffers + Cached + SReclaimable kullanır.

## Disk kullanımı

`df -P -k` çıktısından (boyutlar 1 KiB bloklar hâlinde):

- **Kullanım %** = kullanılan ÷ (kullanılan + boş) — tam olarak `df`'in
  yazdığı değer. root için ayrılan alan (ext4'te genellikle %5) boş sayılmaz;
  bu yüzden bir disk %100 gösterirken `df` root için hâlâ biraz boş alan
  bildirebilir.
- **Kutucuktaki DSK**, tüm gerçek dosya sistemlerinin **en yüksek** kullanım
  yüzdesidir.

Hangi dosya sistemleri "gerçek" sayılır? skry şunları gizler:

- sanal dosya sistemleri: `tmpfs`, `devtmpfs`, `udev`, `shm`, `proc`, `sysfs`,
  `cgroup`, `cgroup2`, `devpts`, `mqueue`, `efivarfs` ve boyutu 0 olanlar
- loop aygıtları (`/dev/loop*`, ör. snap paketleri)
- `/proc`, `/sys`, `/dev`, `/run`, `/snap/`, `/var/lib/docker/`,
  `/var/lib/containers/`, `/var/lib/kubelet/` altındaki bağlama noktaları
- kopyalar: aynı aygıt birden çok kez bağlanmışsa (bind mount), yalnızca en
  kısa bağlama yolu gösterilir

`df`, destekleniyorsa `-l` ile (yalnızca yerel dosya sistemleri) çalıştırılır;
böylece ölü bir NFS sunucusu ölçümü kilitleyemez. Varsa `timeout 10` altında
da çalışır.

## Disk I/O

`/proc/diskstats` içinden, yalnızca bütün diskler için:

| skry'nin gösterdiği | Formül |
| --- | --- |
| Okuma/sn, Yazma/sn | sektör farkı × 512 bayt ÷ saniye |
| IOPS r/w | tamamlanan okuma/yazma farkı ÷ saniye |
| Util % | I/O ile geçen milisaniye ÷ geçen milisaniye |

Bölümler (`sda1`, `nvme0n1p2`, `mmcblk0p1`), loop, ram, zram, nbd, cdrom ve
disket aygıtları ile hiç I/O yapmamış diskler gizlenir. Device-mapper (`dm-0`)
ve yazılım RAID (`md0`) aygıtları gösterilir.

%100'e yakın Util, aygıtın sürekli meşgul olduğu anlamına gelir — tek bir dönen
disk için bu bir darboğazdır; hızlı SSD'ler ve RAID için ise yalnızca bir
ipucudur, çünkü aynı anda birçok isteği karşılayabilirler.

## Ağ

`/proc/net/dev` içinden, arayüz başına:

- **RX/sn, TX/sn** = bayt farkı ÷ saniye.
- Arayüz listesi `lo` (loopback) ve `veth*` (her konteyner için bir tane)
  arayüzlerini gizler.
- **Kutucuktaki ↓ ↑** yalnızca "birincil" arayüzlerin toplamıdır. Trafik iki
  kez sayılmasın diye (bir kez konteynerin sanal arayüzünde, bir kez fiziksel
  arayüzde) konteyner ve sanallaştırma arayüzleri hariç tutulur: `docker*`,
  `br-*`, `virbr*`, `cni*`, `flannel*`, `cali*`, `vnet*`, `tunl*`, `kube-*`,
  `vxlan*`, `genev*`, `podman*`, `lxc*`.

Değerler saniyede **bayt**tır ve ikili birimlerle gösterilir (1 KiB = 1024
bayt). Megabit/saniye için 125 000'e bölün (1 MiB/sn ≈ 8,4 Mbit/sn).

## Yük

- **Yük ortalaması** (1, 5, 15 dakika) doğrudan `/proc/loadavg`'den alınır.
  Çalışan ya da çalışmayı bekleyen (disk beklemesi dahil) süreçlerin ortalama
  sayısıdır.
- **Çekirdek başına yük** = 1 dakikalık yük ÷ çekirdek sayısı. Eşikler bunu
  kullanır, çünkü 8'lik yük 2 çekirdekli bir sunucuda alarm verici, 32
  çekirdekli bir sunucuda ise rahattır. Kabaca: çekirdek başına 1,0'ın altı
  rahattır; sürekli 1,5–2'nin üstü işlerin sırada beklediği anlamına gelir.

## Süreçler

Her süreç için `/proc/<pid>/stat` içinden:

| skry'nin gösterdiği | Formül |
| --- | --- |
| **CPU%** | (utime + stime) farkı ÷ CLK_TCK ÷ saniye × 100 |
| **Mem%** | RSS sayfaları × sayfa boyutu ÷ MemTotal |
| RSS | bayt cinsinden fiziksel bellekte duran kısım |

- **Süreç CPU% değeri tek çekirdeğe göredir**, `top`'taki gibi: iki çekirdeği
  tamamen kullanan bir süreç %200 gösterir. (Yukarıdaki sunucu CPU %'si ise tüm
  çekirdeklere göredir.)
- Yeni süreçler iki kez görülene kadar `…` gösterir.
- Liste, CPU'ya göre en meşgul 12 süreç **artı** belleğe göre en büyük 12
  süreçten oluşur (tekrarlar çıkarılır); böylece boşta duran ama belleği
  şişiren bir süreç de görünür.
- Kullanıcı adı `ps`'ten gelir. BusyBox'ta ya da çok uzun kullanıcı adlarında
  kısalmış veya eksik olabilir.

## Çalışma süresi

`/proc/uptime`; `23d 4h`, `5h 12m`, `7m 3s` biçiminde gösterilir (d=gün,
h=saat, m=dakika, s=saniye).

## Başarısız SSH girişleri

Son 24 saatte sshd'den gelen ve `Failed password`, `Failed publickey`,
`Failed keyboard-interactive` ya da `Invalid user` içeren satırlar. Bilmeniz
gereken iki şey:

- Var olmayan bir kullanıcı için yapılan parola denemesi genellikle **iki**
  satır üretir ("Invalid user…" ve "Failed password for invalid user…") ve bu
  yüzden iki kez sayılır. Sayıyı "kişi" değil, "başarısız kimlik doğrulama
  olayı" olarak düşünün.
- Log dosyalarında (journal değil) 24 saatlik pencere saat bazında
  uygulanır; yani 24–25 saati kapsar.

## Bekleyen güncellemeler

- **Debian/Ubuntu:** `apt-get -s dist-upgrade`'in kuracağı her paket. Kaynağı
  bir `…-security` deposu (ör. `Debian-Security` ya da `noble-security`) olan
  paket **güvenlik** güncellemesi sayılır.
- **Alpine:** `apk version -l '<'` ile listelenen paketler. Alpine'de güvenlik
  meta verisi yoktur; güvenlik sayısı `?` gösterir.
- **RHEL ailesi (isteğe bağlı):** `dnf updateinfo list` içindeki benzersiz paket
  adları; güvenlik = önem derecesi `/Sec.` ile biten duyurular.
