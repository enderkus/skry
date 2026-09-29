# Her ölçümde neler olur

Bir **ölçüm** (tick), tek bir ölçme turudur. Varsayılan olarak her sunucu için
**2 saniyede** bir ölçüm yapılır (ayar: `interval`, seçenek: `--interval`).

## Hızlı ve yavaş bölümler

Her şeyin 2 saniyede bir ölçülmesi gerekmez. Bazı veriler yavaş değişir ya da
toplanması daha pahalıdır; bu yüzden skry betiği iki kısma ayırır:

| Her ölçümde (hızlı) | 60 saniyede bir (yavaş, ayar `slow_interval`) |
| --- | --- |
| Sunucu adı, çekirdek, saat | İşletim sistemi adı ve sürümü |
| CPU sayaçları (`/proc/stat`) | CPU modeli |
| Bellek (`/proc/meminfo`) | IP adresleri |
| Yük (`/proc/loadavg`) | Dinleyen TCP portları |
| Çalışma süresi (`/proc/uptime`) | Docker/Podman konteynerleri |
| Ağ sayaçları (`/proc/net/dev`) | Çökmüş systemd birimleri |
| Disk sayaçları (`/proc/diskstats`) | Son 24 saatteki başarısız SSH girişleri |
| Disk kullanımı (`df`) | Bekleyen paket güncellemeleri |
| Süreç başına CPU ve bellek | |
| Süreç sahipleri (`ps`) | |

"Yavaş" bir ölçümde iki kısım da **aynı tek betikle** gönderilir — ölçüm başına
yine tek komut vardır. Yavaş ölçümler arasında en son bilinen yavaş değerler
saklanır ve gösterilir.

## Bağlandıktan sonraki ilk saniyeler

```text
t = 0 sn   bağlan, host anahtarını doğrula, giriş yap
t ≈ 0 sn   ölçüm 1: bellek, disk kullanımı, yük, süreçler (bellek) görünür
           CPU, ağ hızı, disk I/O ve süreç CPU'su "n/a" / "…" gösterir
t ≈ 1 sn   ölçüm 2 (bilerek erken alınır): hızlar görünür
t ≈ 3 sn   ölçüm 3 ve bundan sonra her 2 saniyede bir
```

Hızlar iki ölçüm gerektirir; bu yüzden skry ikinci ölçümü tam bir aralık
beklemek yerine bir saniye sonra alır.

## Zamanlama ve zaman aşımları

| Sınır | Varsayılan | Aşılırsa ne olur |
| --- | --- | --- |
| `connect_timeout` | atlama başına 10 sn | Bağlantı denemesi "timeout" ile başarısız olur; skry sonra tekrar dener |
| `command_timeout` | 30 sn | Ölçüm başarısız olur; skry bağlantıyı düşürüp yeniden bağlanır |
| Sunucudaki komut başına `timeout` | 10 sn | Yalnızca o komut (ör. takılmış bir NFS bağlamasında `df`) sonlandırılır; bölümü `n/a` gösterir |

Ölçümler asla birikmez: bir ölçüm aralıktan uzun sürerse bir sonraki biraz
geç başlar.

## Aynı anda çok sunucu

Her sunucunun kendi işçisi vardır; farklı sunuculardaki ölçümler paralel
çalışır. `concurrency` ayarı (varsayılan 32), **aynı anda** kaç sunucuya
bağlanıldığını ya da kaç sunucunun ölçüldüğünü sınırlar. 500 sunucu ve 32
eşzamanlılıkla bile bir tur hızlıdır, çünkü her ölçüm sunucuda yalnızca birkaç
milisaniye sürer.

## Her ölçümden sonra ne saklanır

- Yeni değerler bellektekilerin yerini alır; tüm görünümler güncellenir.
- Tek satırlık bir özet (durum, CPU, bellek, disk, yük, ağ) yerel geçmiş
  veritabanına yazılır.
- Her 30 saniyede (`history.detail_interval`) sunucunun tam durumu (süreçler,
  diskler, konteynerler, …) da zamanda yolculuk için saklanır.
- Sağlık, baseline'lar ve uyarı koşulları yeniden değerlendirilir.
