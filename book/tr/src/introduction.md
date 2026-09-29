# Giriş

**skry**, Linux sunucularınızın ne yaptığını gösterir: CPU, bellek, diskler,
ağ, süreçler, konteynerler, çökmüş servisler, güvenlik sorunları… Tek bir
sunucu için de, aynı anda yüz sunucu için de — **sunuculara hiçbir şey
kurmadan**.

Zaten her gün yaptığınız gibi düz SSH ile bağlanır, küçük ve salt okunur bir
kabuk betiği çalıştırır, gelen cevabı okur ve ekranınıza çizer. skry'yi
kapattığınızda sunucularda arkasında hiçbir iz kalmaz.

![Terminalde skry](https://raw.githubusercontent.com/enderkus/skry/main/docs/demo.gif)

## Kimler için?

- **Birkaç taneden birkaç yüze kadar Linux sunucusu yönetiyorsanız** ve hangisinin
  sağlıklı, hangisinin zorlandığını, hangisinin çöktüğünü bir bakışta görmek
  istiyorsanız.
- **İzleme ajanı kuramıyor ya da kurmak istemiyorsanız:** sunucular bir
  müşteriye ait olabilir, değişiklik yönetimi katı olabilir ya da sadece iki
  dakikada çalışan bir şey istiyor olabilirsiniz.
- **Bir olayı (incident) çözmeye çalışıyorsanız** ve tüm filoyu hemen canlı
  görmeniz, sonrasında da durumun yazılı bir kaydını almanız gerekiyorsa.
- **Hızlı cevaplar istiyorsanız:** "5432 portunu hangi sunucular dinliyor?",
  "nginx nerede çalışıyor?", "hangi sunucularda bekleyen güvenlik güncellemesi
  var?" gibi.

## skry ne değildir?

- **Tam teşekküllü bir izleme platformunun yerine geçmez.** Aylarca veri
  saklamaz, her metrik için pano sunmaz, nöbet çizelgesi yönetmez. skry
  varsayılan olarak 24 saatlik geçmiş tutar ve "şu an"a odaklanır. Yine de
  elinizde Prometheus varsa onu besleyebilir.
- **Uzaktan yönetim aracı değildir.** skry bir sunucuda asla hiçbir şeyi
  değiştirmez ve filoda komut çalıştırma özelliğine bilerek sahip değildir.
- **Ajan değildir.** İki ölçüm arasında sunucularınızda hiçbir şey çalışmaz.

## Üç söz

1. **Ajansız.** Sunuculara hiçbir şey kurulmaz, kopyalanmaz, yazılmaz. Geçici
   dosya yok, sudo yok.
2. **Salt okunur.** skry yalnızca `/proc/meminfo` gibi dosyaları okur ve `df`,
   `ps` gibi zararsız inceleme komutları çalıştırır.
3. **Varsayılan olarak güvenli.** skry sunucuların kimliğini `known_hosts`
   dosyanızla doğrular; tanımadığı sunucuları siz açıkça izin vermedikçe
   reddeder. Parolaları asla saklamaz.

## Bu dokümanı nasıl okumalı?

- skry'de yeni misiniz? **Başlarken** bölümünü baştan sona okuyun. "SSH ile
  sunucularıma girebiliyorum" dışında hiçbir ön bilgi varsaymaz. Bu bile yeniyse
  [SSH temelleri](ssh-basics.md) sayfası anlatır.
- skry'nin sunucularınızda tam olarak ne yaptığını ve her sayının nasıl
  hesaplandığını anlamak mı istiyorsunuz? **skry nasıl çalışır** bölümünü okuyun.
- Belirli bir özelliği mi arıyorsunuz? **skry'yi kullanmak** bölümüne atlayın.
- Bir şey çalışmıyor mu? [Sorun giderme](troubleshooting.md) ve
  [SSS](faq.md) sayfalarına gidin.

Her sayfayı sağ üstteki **English** bağlantısıyla İngilizce okuyabilirsiniz.
