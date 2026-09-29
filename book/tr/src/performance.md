# Performans ve yük

## Sunucularınızda

Her ölçümde (varsayılan 2 sn'de bir) kısa bir betik çalışır. Birkaç
milisaniyede biten birkaç minik süreç (`sh`, `cat`, `grep`, `awk`, `df`, `ps`)
başlatır. Ölçümler arasında yerleşik bir süreç yoktur.

60 saniyede bir yavaş bölümler birkaç komut daha ekler. En pahalısı
`docker stats --no-stream`'dir; Docker tarafında yaklaşık iki saniye sürer
(konteyner kullanımını örnekler). Docker/Podman olmayan sunucularda maliyeti
sıfırdır.

## Ağda

- Sunucu başına **tek** SSH bağlantısı, açık tutulur.
- Ölçüm başına gönderilen komut: yaklaşık **1,4 KB** (yavaş ölçümlerde yaklaşık
  **4,2 KB**).
- Ölçüm başına cevap: birkaç KB temel veri artı çalışan süreç başına kabaca
  **60 bayt**. 300 süreçli bir sunucu ölçüm başına yaklaşık 20–25 KB döndürür;
  bu, varsayılan aralıkla saniyede yaklaşık 10–12 KB demektir. Birkaç süreçli
  küçük bir konteyner yaklaşık 5 KB döndürür.
- Artı her 15 saniyede bir SSH keepalive.

Trafiği azaltmak için aralığı artırın (`--interval 10`).

## Sizin bilgisayarınızda

- Sunucu başına hafif bir görev; bir ölçümü ayrıştırmak mikrosaniyeler sürer.
- Bellek, sunucu sayısıyla ve sunucu başına veri miktarıyla (süreçler,
  konteynerler) büyür; her sunucu bellekte yalnızca en son durumunu tutar.
- Geçmiş yazmaları arka plandaki bir iş parçacığında küçük işlemlerle yapılır.

## Boyutlandırma önerileri

| Filo | Önerilen ayarlar |
| --- | --- |
| 1–50 sunucu | varsayılanlar (2 sn aralık, 32 eşzamanlılık) |
| 50–300 sunucu | `interval = 5`, `concurrency = 64` |
| 300+ sunucu | `interval = 10`–`30`, `concurrency = 64`–`128`; farklı skry örneklerinin izlediği gruplara bölmeyi düşünün |

`concurrency`, sunucu sayısını değil, eşzamanlı bağlanma/ölçümleri sınırlar.
Büyük bir filoda ölçümler geride kalmaya başlarsa artırın; bastion'ınız ya da
VPN'iniz çok sayıda paralel bağlantıda zorlanıyorsa azaltın (bastion
arkasındaki her sunucu onun içinden bir tüneldir).

Çok büyük filolarda dosya tanıtıcı (file descriptor) sınırınız önemlidir: her
sunucu en az bir TCP bağlantısı kullanır (bastion'la iki). 1024'lük
`ulimit -n` birkaç yüz sunucu için yeterlidir.
