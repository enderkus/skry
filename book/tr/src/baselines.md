# Baseline'lar: "bu sunucu için olağandışı"

Sabit eşikler "CPU %80'in üstünde mi?" sorusunu cevaplar. Ama %70 CPU bir
derleme sunucusu için normaldir, genellikle %3'te boşta duran bir DNS sunucusu
için ise alarm vericidir. **Baseline'lar** her sunucu için neyin normal
olduğunu öğrenir ve *o sunucu için* olağandışı olan değerleri işaretler.

## Sade bir dille fikir

skry her sunucu ve her metrik için sürekli güncellenen iki sayı tutar:

- **olağan değer** (hareketli ortalama) ve
- **olağan değişkenlik** (hareketli standart sapma).

Yeni bir değer geldiğinde skry şunu sorar: *bu, olağan değerin kaç "olağan
değişkenlik" üstünde?* Bu sayı **z-skorudur**.

```text
dns1 için olağan CPU: %3, olağan değişkenlik ±%1,5
yeni değer: %22
z = (22 − 3) / 5 = 3,8      ← CPU için değişkenlik en az 5 puan alınır (aşağıya bakın)
3,8 > 3,5 → sapma: "cpu 22.0% vs usual 3.0% (z=3.8)"
```

Bir sapma, hiçbir eşik aşılmıyorsa sunucuyu **camgöbeği** (`deviation`)
yapar, kutucukta `◆ cpu z3.8` gösterir, ayrıntılarda *sorunlar* altında
görünür ve uyarı gönderebilir.

## Hangi metrikler

| Metrik | Kullanılan en düşük değişkenlik |
| --- | --- |
| CPU % | 5 yüzde puanı |
| bellek % | 2 yüzde puanı |
| çekirdek başına yük | 0,25 |
| en dolu disk % | 1 yüzde puanı |
| ağdan alınan | olağan değerin %25'i, en az 64 KiB/sn |
| ağa gönderilen | olağan değerin %25'i, en az 64 KiB/sn |

En düşük değişkenlik, hiç kıpırdamayan metriklerin minik oynamalarda dev
z-skorları üretmesini engeller (her zaman tam %41'de duran bir disk aksi hâlde
%42'yi işaretlerdi).

**Yalnızca yukarı yönlü sapmalar bildirilir.** Birden sessizleşen bir sunucu
nadiren bir olaydır; gerçek kesintileri eşikler ve erişilebilirlik kontrolü
yakalar.

## Isınma

Bir baseline, `baseline.warmup` kadar ölçüm görmeden güvenilir sayılmaz
(varsayılan **300**; varsayılan 2 saniyelik aralıkla 10 dakika). Isınma
süresince o sunucu için sapma bildirilmez.

Baseline'lar geçmiş veritabanına kaydedilir; skry yeniden başladığında
kaldığı yerden devam eder ve yeniden ısınması gerekmez. Geçmiş kapalıysa her
başlangıç sıfırdan başlar.

## Günün saati

Birçok sunucunun günlük ritmi vardır: 03:00'te yedekleme, öğlen iş trafiği.
`baseline.hourly = true` (varsayılan) ile skry genel baseline'a ek olarak
**günün her saati için ayrı bir baseline** tutar:

- Şu anki saatin baseline'ı ısınmasını tamamladıysa değer onunla
  karşılaştırılır: her gece oluyorsa 03:00'teki %80 CPU normaldir.
- Değilse genel baseline kullanılır.

Her saat yalnızca o saat içinde ölçüm aldığı için saatlik baseline'ların
dolması birkaç gün sürer. Saat, sizin bilgisayarınızın yerel saatidir.

## Ne kadar hızlı uyum sağlar

Hareketli ortalama bir yumuşatma katsayısı kullanır: `baseline.alpha`
(varsayılan 0,02); her yeni ölçümün %2 söz hakkı vardır. Kabaca, genel
baseline son 50–100 ölçümü, yani varsayılan aralıkla birkaç dakikayı
"hatırlar" — saatlik baseline'lar buna karşılık birkaç günü. Yeni bir normal
(örneğin belleği kalıcı olarak artıran bir sürüm) bir süre sonra işaretlenmez
olur, çünkü sapan değerler de baseline'a katılır.

## Ayar yapmak

```toml
[baseline]
enabled = true
alpha = 0.02        # yüksek = daha hızlı uyum, daha hızlı unutma
z_threshold = 3.5   # yüksek = daha az ama daha anlamlı sapma
warmup = 300        # karar vermeden önceki ölçüm sayısı
hourly = true
```

- Çok fazla camgöbeği kutucuk mu var? `z_threshold`'u 4 ya da 5'e çıkarın.
- Kalıcı bir değişiklikten sonra sapmalar çok uzun süre işaretli mi kalıyor?
  `alpha`'yı artırın (ör. 0,05).
- Bunu hiç istemiyor musunuz? `enabled = false`.
