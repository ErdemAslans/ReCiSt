# ReCiSt: Biyolojiden Esinlenmiş Ajantik Kendi Kendini İyileştiren Framework

## Vibe Coding Context Prompt

Bu doküman, ReCiSt projesini sıfırdan geliştirmek için gereken tüm bağlamı içerir. Kod içermez, sadece ne yapılacağını detaylıca açıklar. Bu prompt ile bir yapay zeka veya geliştirici projeyi tam olarak anlayıp kodlayabilir.

---

## Bölüm 1: Projenin Özü

### 1.1 Problem Tanımı

Modern dağıtık sistemlerde (özellikle Kubernetes kümeleri) hatalar kaçınılmazdır. Şu an bu hatalar şöyle yönetiliyor:

- Bir pod çöktüğünde alarm çalar
- Nöbetçi mühendis uyanır
- Manuel olarak log'lara bakar
- Sorunu teşhis eder
- Manuel olarak müdahale eder
- Sistem düzelir

Bu süreç ortalama 30-60 dakika sürüyor. Gece yarısı olduğunda mühendis yorgun, hata yapma olasılığı yüksek.

### 1.2 Çözüm: Otomatik Kendi Kendini İyileştirme

ReCiSt, insan vücudunun yara iyileştirme mekanizmasından esinlenerek dört aşamalı otomatik iyileştirme sistemi sunuyor:

- Hata tespit edilir ve izole edilir (kanama durdurulur)
- Log'lar analiz edilir, kök neden bulunur (bağışıklık sistemi devreye girer)
- Çözüm stratejisi belirlenir ve uygulanır (yeni doku oluşur)
- Öğrenilenler kaydedilir (doku güçlenir, hafıza oluşur)

Hedef: 30-60 saniyede otomatik iyileşme, mühendis müdahalesi olmadan.

### 1.3 Neden Biyoloji Benzetmesi?

İnsan vücudu milyonlarca yıllık evrimle mükemmelleşmiş bir kendi kendini iyileştirme sistemine sahip. Elini kestiğinde doktora gitmene gerek yok, vücut otomatik olarak:

1. Kanamayı durdurur (trombositler pıhtı oluşturur)
2. Enfeksiyonu önler (bağışıklık hücreleri gelir)
3. Yarayı kapatır (yeni hücreler oluşur)
4. Dokuyu güçlendirir (kolajen yeniden düzenlenir)

Bu süreç:
- Otomatik (bilinçli düşünme gerektirmez)
- Paralel (birden fazla aşama eş zamanlı çalışır)
- Öğrenen (aşı mantığı - bir kez karşılaşılan tehdit hatırlanır)
- Ölçeklenebilir (küçük kesik de büyük yara da aynı mekanizma)

ReCiSt bu prensipleri yazılım sistemlerine uyguluyor.

---

## Bölüm 2: Biyolojik Aşamalar ve Sistem Karşılıkları

### 2.1 Hemostaz (Kanama Durdurma) → Sınırlama Katmanı

#### Biyolojik Süreç
Bir yara oluştuğunda ilk tepki kanın dışarı akmasını durdurmaktır. Trombositler (kan pulcukları) yara bölgesine toplanır ve pıhtı oluşturur. Bu pıhtı bir bariyer görevi görür, hem kan kaybını durdurur hem de dışarıdan mikrop girişini engeller. Damarlar kasılır (vazokonstriksiyon) ve kan akışı yavaşlar.

#### Sistem Karşılığı: Sınırlama Ajanı
Bir pod veya servis hata verdiğinde ilk yapılması gereken hatanın yayılmasını durdurmaktır. Sınırlama Ajanı şunları yapar:

**Hata Tespiti:**
- Prometheus metriklerini sürekli izler
- CPU, bellek, ağ gecikmesi için eşik değerler tanımlanır
- Eşik aşıldığında alarm tetiklenir
- Kubernetes olayları (events) dinlenir
- Pod durumları kontrol edilir (CrashLoopBackOff, OOMKilled, vb.)

**İzolasyon Mekanizmaları:**
- NetworkPolicy uygulanır (hatalı pod'a gelen/giden trafik kesilir)
- Service endpoint'lerinden çıkarılır (yük dengeleyici artık trafik göndermez)
- Devre kesici (circuit breaker) aktif edilir
- Diğer sağlam pod'lara trafik yönlendirilir

**Komşu Müzakeresi:**
- Sağlam komşu pod'lar belirlenir
- Her birine "bu işi üstlenebilir misin?" sorulur
- Komşunun kapasitesi (CPU, bellek) kontrol edilir
- Kabul eden komşulara görev dağıtılır
- Geçici yönlendirme tablosu oluşturulur

**Çıktılar:**
- Hata kümesi F(t): Hatalı olarak işaretlenen pod'ların listesi
- İzolasyon kuralları: Uygulanan NetworkPolicy'ler
- Yönlendirme tablosu: Hangi trafik nereye gidecek

**Kritik Süre Hedefi:** Hata tespitinden izolasyona kadar maksimum 5 saniye

### 2.2 İltihap (Enflamasyon) → Teşhis Katmanı

#### Biyolojik Süreç
Pıhtı oluştuktan sonra bağışıklık sistemi devreye girer. Beyaz kan hücreleri (lökositler) yara bölgesine göç eder. Burada iki temel görev yaparlar: hasarlı hücreleri ve yabancı maddeleri temizlemek (fagositoz) ve enfeksiyona neden olabilecek mikropları yok etmek. Bu süreçte yara bölgesi kızarır, şişer ve ısınır - bunlar iyileşmenin işaretleridir.

#### Sistem Karşılığı: Teşhis Ajanı
İzolasyon yapıldıktan sonra "tam olarak ne oldu?" sorusunun cevabını bulmak gerekir. Teşhis Ajanı şunları yapar:

**Veri Toplama:**
- Hatalı pod'un son N dakikadaki log'ları Loki'den çekilir
- Sistem log'ları: Kernel mesajları, OOM killer, disk I/O hataları
- Uygulama log'ları: Exception'lar, hata mesajları, stack trace'ler
- Ağ log'ları: Bağlantı zaman aşımları, DNS hataları, TLS sorunları
- Prometheus'tan aynı zaman dilimindeki metrikler alınır
- Kubernetes event'leri sorgulanır

**Log Dönüşümü:**
- Ham log satırları yapılandırılmış varlıklara dönüştürülür
- Her varlık: zaman damgası, seviye, kaynak, mesaj, bağlam
- Benzer log'lar gruplandırılır (deduplication)
- Zaman serisi oluşturulur (ne zaman başladı, ne zaman yoğunlaştı)

**Nedensel Alt Ağaç Oluşturma:**
- Her gözlemlenen varlık bir düğüm olur
- Düğümler arasında nedensellik ilişkileri kurulur
- Örnek: "DB bağlantı havuzu tükendi" → "Sorgu zaman aşımı" → "API 500 hatası"
- Bu graf yapısı kök nedene giden yolu gösterir

**LLM Destekli Analiz:**
- Toplanan log'lar ve metrikler LLM'e gönderilir
- Prompt şablonu: "Bu log'lara ve metriklere göre sistemde ne oldu? Kök neden nedir?"
- LLM doğal dil anlama yeteneğiyle pattern'leri tanır
- Çıktı: Teşhis hipotezi ve güven skoru

**Çıktılar:**
- Nedensel alt ağaç: Hatanın oluşum zincirini gösteren graf
- Teşhis hipotezi: "Veritabanı bağlantı havuzu tükendi, connection leak var"
- Kök neden: En olası temel sebep
- Kanıt listesi: Hipotezi destekleyen log satırları ve metrikler

**Kritik Süre Hedefi:** Log toplama + analiz maksimum 15 saniye

### 2.3 Çoğalma (Proliferasyon) → Üst-Bilişsel Katman

#### Biyolojik Süreç
Enflamasyon aşamasında temizlik yapıldıktan sonra yeniden yapılanma başlar. Fibroblast hücreleri yara bölgesine göç eder ve kolajen üretmeye başlar. Yeni kan damarları oluşur (anjiogenez). Yara kenarlarından epitel hücreleri çoğalarak yarayı kapatmaya başlar. Bu aşamada vücut birden fazla mekanizmayı paralel olarak çalıştırır ve en etkili olanı öne çıkarır.

#### Sistem Karşılığı: Üst-Bilişsel Ajan
Teşhis konulduktan sonra "nasıl düzelteceğiz?" sorusunun cevabı aranır. Üst-Bilişsel Ajan şunları yapar:

**Strateji Üretimi:**
- Teşhis hipotezine göre olası çözümler listelenir
- Her çözüm için artılar ve eksiler değerlendirilir
- Risk seviyesi hesaplanır (bu çözüm başka sorunlara yol açar mı?)
- Uygulama süresi tahmin edilir

**Mikro-Ajan Mimarisi:**
- Her olası çözüm yolu için bir mikro-ajan oluşturulur
- Mikro-ajanlar paralel olarak çalışır
- Her mikro-ajan kendi hipotezini test eder
- Örnek mikro-ajanlar:
  - Yeniden başlatma mikro-ajanı: Pod restart'ın çözüp çözmeyeceğini değerlendirir
  - Ölçekleme mikro-ajanı: Replica sayısını artırmanın etkisini hesaplar
  - Yapılandırma mikro-ajanı: Config değişikliğinin gerekliliğini analiz eder
  - Kaynak mikro-ajanı: CPU/bellek limitlerini ayarlamanın sonuçlarını öngörür

**Akıl Yürütme Döngüsü:**
- Her mikro-ajan LLM'e "bu çözüm işe yarar mı?" diye sorar
- LLM kanıtlara bakarak güven skoru verir
- Güven skoru eşiğin altındaysa mikro-ajan daha fazla kanıt toplar
- Döngü güven skoru yeterince yüksek olana kadar devam eder
- Maksimum derinlik sınırı vardır (sonsuz döngüyü önler)

**Karar Mekanizması:**
- Tüm mikro-ajanların sonuçları toplanır
- En yüksek güven skorlu çözüm seçilir
- Birden fazla çözümün kombinasyonu da olabilir
- Seçilen çözüm için uygulama planı oluşturulur

**Uygulama:**
- Kubernetes API üzerinden eylemler gerçekleştirilir
- Pod yeniden başlatma: kubectl delete pod
- Ölçekleme: kubectl scale deployment
- Yapılandırma değişikliği: kubectl patch configmap
- Kaynak ayarlama: kubectl patch deployment (limits)
- NetworkPolicy kaldırma: İzolasyonu sonlandır

**Çıktılar:**
- Seçilen çözüm stratejisi
- Uygulama planı (hangi sırayla ne yapılacak)
- Uygulama sonucu (başarılı/başarısız)
- Geri alma planı (rollback) - çözüm işe yaramazsa ne yapılacak

**Kritik Süre Hedefi:** Strateji belirleme + uygulama maksimum 20 saniye

### 2.4 Yeniden Yapılanma (Remodeling) → Bilgi Katmanı

#### Biyolojik Süreç
Yara kapandıktan sonra bile iyileşme devam eder. Kolajen lifleri yeniden düzenlenir ve güçlenir. Yeni oluşan damarlar olgunlaşır. En önemlisi, bağışıklık sistemi bu deneyimi "hatırlar" - aynı patojenle tekrar karşılaşıldığında çok daha hızlı tepki verir (aşı prensibi). Bu hafıza hem lokal (doku seviyesinde) hem de sistemik (tüm vücut) olarak saklanır.

#### Sistem Karşılığı: Bilgi Ajanı
İyileşme tamamlandıktan sonra öğrenilenler kaydedilir ve gelecekte kullanılmak üzere saklanır. Bilgi Ajanı şunları yapar:

**Olay Kaydı:**
- Tüm iyileşme süreci dokümante edilir
- Zaman damgaları: Hata tespiti, izolasyon, teşhis, uygulama, iyileşme
- Uygulanan çözüm ve sonucu
- Kullanılan kaynaklar (CPU, bellek)
- Toplam iyileşme süresi

**Vektör Dönüşümü:**
- Olay özeti embedding'e dönüştürülür
- Log pattern'leri vektörleştirilir
- Teşhis hipotezi vektörleştirilir
- Bu vektörler benzerlik aramasını mümkün kılar

**Depolama Mimarisi:**

Yerel Buluşma Noktası (Yerel BN):
- Her namespace için ayrı önbellek
- Son N olayı tutar
- Hızlı erişim için bellek içi (Redis)
- Sık karşılaşılan pattern'ler burada

Küresel Buluşma Noktası (Küresel BN):
- Tüm küme genelinde merkezi depo
- Vektör veritabanı (Qdrant)
- Tüm geçmiş olaylar burada
- Cross-namespace öğrenme mümkün

**Konu Bölümleme:**
- Benzer hatalar aynı konuya (topic) atanır
- Örnek konular: "veritabanı sorunları", "bellek sızıntıları", "ağ hataları"
- Yeni bir hata geldiğinde önce konusu belirlenir
- O konudaki geçmiş çözümler öncelikli kontrol edilir

**Proaktif Kullanım:**
- Yeni bir hata geldiğinde önce bilgi tabanı sorgulanır
- "Bu hataya benzer bir şey daha önce oldu mu?"
- Benzer olay bulunursa o zaman işe yarayan çözüm önerilir
- Bu sayede teşhis ve çözüm süresi kısalır

**Bağlamsal Kayma Tespiti:**
- Sistem zamanla değişir (yeni servisler, artan yük)
- Eski çözümler artık işe yaramayabilir
- Bilgi Ajanı çözümlerin güncelliğini takip eder
- Eskiyen bilgiler güncellenir veya silinir

**Çıktılar:**
- Güncellenmiş bilgi tabanı
- Konu-bölüm haritası
- Benzerlik indeksi
- Proaktif tahminler (yakında şu hata olabilir)

**Kritik Süre Hedefi:** Kayıt ve indeksleme maksimum 5 saniye

---

## Bölüm 3: Sistem Mimarisi

### 3.1 Genel Görünüm

Sistem bir Kubernetes Operatörü olarak çalışır. Operatör, Kubernetes API'yi dinleyen ve özel kaynakları (Custom Resource) yöneten bir kontrolcüdür. ReCiSt operatörü şu bileşenlerden oluşur:

**Ana Kontrolcü:**
- Kubernetes API'yi dinler
- Olayları ilgili ajana yönlendirir
- Ajanlar arası koordinasyonu sağlar
- Durum yönetimini yapar

**Dört Ana Ajan:**
- Sınırlama Ajanı: Hata tespiti ve izolasyon
- Teşhis Ajanı: Log analizi ve kök neden bulma
- Üst-Bilişsel Ajan: Çözüm stratejisi ve uygulama
- Bilgi Ajanı: Öğrenme ve hafıza

**Yardımcı Bileşenler:**
- LLM Motoru: Claude/GPT/Gemini API ile iletişim
- Gözlemlenebilirlik İstemcisi: Prometheus ve Loki ile iletişim
- Kubernetes İstemcisi: K8s API ile iletişim
- Vektör Veritabanı İstemcisi: Qdrant ile iletişim
- Önbellek İstemcisi: Redis ile iletişim

### 3.2 Veri Akışı

Bir hata oluştuğunda veri şu şekilde akar:

```
[Kubernetes Kümesi]
        │
        ▼
[Prometheus - Metrikler] ──────────────────┐
[Loki - Log'lar] ──────────────────────────┤
[K8s API - Olaylar] ───────────────────────┤
        │                                   │
        ▼                                   │
┌─────────────────────────────────────────────────┐
│              ReCiSt Operatörü                   │
│                                                 │
│  [Sınırlama Ajanı]                             │
│         │                                       │
│         │ hata_kümesi                          │
│         ▼                                       │
│  [Teşhis Ajanı] ◄──── [LLM Motoru]            │
│         │                                       │
│         │ nedensel_alt_ağaç + hipotez          │
│         ▼                                       │
│  [Üst-Bilişsel Ajan] ◄──── [Mikro-Ajanlar]    │
│         │                                       │
│         │ çözüm_stratejisi                     │
│         ▼                                       │
│  [Bilgi Ajanı] ◄───► [Qdrant + Redis]         │
│                                                 │
└─────────────────────────────────────────────────┘
        │
        ▼
[Kubernetes API - İyileştirme Eylemleri]
        │
        ▼
[Pod restart, Scale, NetworkPolicy, vb.]
```

### 3.3 Ajanlar Arası İletişim

Ajanlar birbirleriyle olay bazlı (event-driven) iletişim kurar:

**Olay Türleri:**
- HataTespit: Sınırlama Ajanı yayınlar, Teşhis Ajanı dinler
- TeşhisTamamlandı: Teşhis Ajanı yayınlar, Üst-Bilişsel Ajan dinler
- İyileşmeTamamlandı: Üst-Bilişsel Ajan yayınlar, Bilgi Ajanı dinler
- BilgiGüncellendi: Bilgi Ajanı yayınlar, tüm ajanlar dinler

**Olay Yapısı:**
- Olay ID'si (benzersiz tanımlayıcı)
- Olay türü
- Zaman damgası
- Kaynak ajan
- Hedef ajan(lar)
- Veri yükü (payload)

**İletişim Kanalı:**
- Bellek içi kanal (in-memory channel) kullanılır
- Rust'ta tokio::sync::broadcast veya mpsc
- Asenkron iletişim (non-blocking)

### 3.4 Durum Yönetimi

Her iyileştirme süreci bir durum makinesi olarak modellenir:

**Durumlar:**
- Beklemede: Hata henüz tespit edilmedi
- Sınırlanıyor: Hata tespit edildi, izolasyon yapılıyor
- Teşhis Ediliyor: Log'lar analiz ediliyor
- İyileştiriliyor: Çözüm uygulanıyor
- Doğrulanıyor: Çözümün işe yarayıp yaramadığı kontrol ediliyor
- Tamamlandı: İyileştirme başarılı
- Başarısız: İyileştirme başarısız, manuel müdahale gerekli

**Durum Geçişleri:**
- Beklemede → Sınırlanıyor: Eşik aşıldığında
- Sınırlanıyor → Teşhis Ediliyor: İzolasyon tamamlandığında
- Teşhis Ediliyor → İyileştiriliyor: Hipotez oluştuğunda
- İyileştiriliyor → Doğrulanıyor: Eylem uygulandığında
- Doğrulanıyor → Tamamlandı: Metrikler normale döndüğünde
- Doğrulanıyor → Başarısız: Zaman aşımı veya hata devam ederse
- Herhangi → Başarısız: Kritik hata oluştuğunda

**Durum Kalıcılığı:**
- Her durum geçişi Kubernetes CRD'ye yazılır
- Operatör yeniden başlarsa kaldığı yerden devam eder
- Audit log oluşturulur

---

## Bölüm 4: Teknoloji Seçimleri

### 4.1 Programlama Dili: Rust

**Neden Rust:**
- Bellek güvenliği: Sistemin kendisi hata yapmamalı
- Düşük gecikme: Mikrosaniye seviyesinde yanıt süresi
- Eşzamanlılık: Tokio ile güçlü async runtime
- Kubernetes ekosistemi: kube-rs olgun ve aktif
- Kariyer değeri: Rust + K8s + AI çok nadir kombinasyon

**Rust Bileşenleri:**
- kube-rs: Kubernetes API istemcisi ve operatör framework'ü
- tokio: Asenkron runtime
- reqwest: HTTP istemcisi (LLM API çağrıları için)
- serde: Serileştirme/deserileştirme
- tracing: Yapılandırılmış loglama
- qdrant-client: Vektör veritabanı istemcisi

### 4.2 Kubernetes Entegrasyonu

**Operatör Modeli:**
- Custom Resource Definition (CRD) tanımlanır
- Kontrolcü bu CRD'yi izler
- Reconciliation loop ile istenen durum sağlanır

**CRD: SelfHealingPolicy:**
- Hangi namespace'ler izlenecek
- Eşik değerleri (CPU, bellek, gecikme)
- İzin verilen eylemler (restart, scale, vb.)
- LLM yapılandırması
- Bildirim ayarları

**CRD: HealingEvent:**
- Bir iyileştirme olayının kaydı
- Başlangıç zamanı, bitiş zamanı
- Uygulanan eylemler
- Sonuç (başarılı/başarısız)

**Kubernetes API Kullanımı:**
- Pod listesi ve durumu sorgulama
- Pod silme (restart için)
- Deployment ölçekleme
- ConfigMap/Secret güncelleme
- NetworkPolicy oluşturma/silme
- Event oluşturma (audit için)

### 4.3 Gözlemlenebilirlik

**Prometheus:**
- Metrik toplama ve sorgulama
- PromQL ile eşik kontrolü
- Örnek sorgular:
  - CPU kullanımı: container_cpu_usage_seconds_total
  - Bellek kullanımı: container_memory_usage_bytes
  - Ağ gecikmesi: http_request_duration_seconds
  - Hata oranı: http_requests_total{status=~"5.."}

**Loki:**
- Log toplama ve sorgulama
- LogQL ile pattern arama
- Örnek sorgular:
  - Hata log'ları: {namespace="default"} |= "error"
  - Exception'lar: {app="myapp"} |~ "Exception|Error"
  - Belirli zaman aralığı: Son 5 dakika

**Entegrasyon:**
- Prometheus HTTP API ile sorgu
- Loki HTTP API ile sorgu
- Sonuçlar yapılandırılmış veri olarak parse edilir

### 4.4 LLM Entegrasyonu

**Desteklenen LLM'ler:**
- Claude (Anthropic): Birincil tercih
- GPT (OpenAI): Alternatif
- Gemini (Google): Alternatif
- Yerel LLM (Ollama): Hava boşluklu (air-gapped) ortamlar için

**API İletişimi:**
- HTTP POST istekleri
- JSON formatında mesajlar
- Streaming yanıt desteği (opsiyonel)
- Hata yönetimi ve retry mantığı

**Prompt Şablonları:**

Teşhis için:
```
Sen deneyimli bir SRE mühendisisin. Aşağıdaki log'ları ve metrikleri analiz et.

LOG'LAR:
{log_içeriği}

METRİKLER:
{metrik_içeriği}

SORU: Bu sistemde ne oldu? Kök neden nedir?

Yanıtını şu formatta ver:
- Kök Neden: [tek cümle]
- Güven Skoru: [0-100 arası]
- Kanıtlar: [destekleyen log satırları]
```

Çözüm için:
```
Teşhis: {teşhis_hipotezi}

Bu sorun için olası çözümleri değerlendir:
1. Pod yeniden başlatma
2. Replica sayısını artırma
3. Kaynak limitlerini artırma
4. Yapılandırma değişikliği

Her çözüm için:
- Başarı olasılığı
- Risk seviyesi
- Uygulama süresi
```

**Yanıt Parse Etme:**
- JSON formatında yapılandırılmış yanıt beklenir
- Güven skoru çıkarılır
- Önerilen eylem çıkarılır

### 4.5 Bilgi Deposu

**Qdrant (Vektör Veritabanı):**
- Olay embedding'lerini saklar
- Benzerlik araması yapar
- Collection: healing_events
- Vektör boyutu: 1536 (OpenAI) veya 1024 (Claude)

**Redis (Önbellek):**
- Yerel BN için
- Son N olayı tutar
- Hızlı erişim
- TTL ile otomatik temizlik

**Veri Modeli:**

Olay kaydı:
- id: Benzersiz tanımlayıcı
- namespace: Kubernetes namespace
- pod_name: Etkilenen pod
- error_type: Hata kategorisi
- diagnosis: Teşhis hipotezi
- solution: Uygulanan çözüm
- success: Başarılı mı
- duration_ms: Toplam süre
- timestamp: Zaman damgası
- embedding: Vektör temsili

---

## Bölüm 5: Detaylı Ajan Tasarımları

### 5.1 Sınırlama Ajanı

**Sorumluluklar:**
- Prometheus metriklerini periyodik olarak sorgula
- Kubernetes olaylarını dinle
- Eşik ihlallerini tespit et
- Hatalı pod'ları izole et
- Komşu pod'larla müzakere yap
- Trafik yönlendirmesini güncelle

**Yapılandırma Parametreleri:**
- cpu_threshold: CPU kullanım eşiği (örn: 0.9)
- memory_threshold: Bellek kullanım eşiği (örn: 0.85)
- latency_threshold_ms: Gecikme eşiği (örn: 500)
- error_rate_threshold: Hata oranı eşiği (örn: 0.05)
- check_interval_seconds: Kontrol aralığı (örn: 10)

**Algoritma:**

```
her check_interval_seconds saniyede bir:
    metrikler = prometheus.sorgula(tüm_pod_metrikleri)
    
    her pod için:
        eğer pod.cpu > cpu_threshold:
            hata_kümesi.ekle(pod, "yüksek_cpu")
        eğer pod.memory > memory_threshold:
            hata_kümesi.ekle(pod, "yüksek_bellek")
        eğer pod.latency > latency_threshold_ms:
            hata_kümesi.ekle(pod, "yüksek_gecikme")
        eğer pod.error_rate > error_rate_threshold:
            hata_kümesi.ekle(pod, "yüksek_hata_oranı")
    
    eğer hata_kümesi boş değilse:
        her hatalı_pod için:
            izole_et(hatalı_pod)
            sağlam_komşular = bul_sağlam_komşular(hatalı_pod)
            müzakere_et(hatalı_pod, sağlam_komşular)
        
        yayınla(HataTespit olayı, hata_kümesi)
```

**İzolasyon Stratejileri:**

Yumuşak İzolasyon:
- Service endpoint'lerinden çıkar
- Yeni trafik gelmez
- Mevcut bağlantılar tamamlanır

Sert İzolasyon:
- NetworkPolicy ile tüm trafiği kes
- Hemen etkili
- Mevcut bağlantılar da kesilir

Seçim Kriteri:
- Hata oranı çok yüksekse (>0.5): Sert izolasyon
- Aksi halde: Yumuşak izolasyon

### 5.2 Teşhis Ajanı

**Sorumluluklar:**
- Hatalı pod'un log'larını topla
- Metrikleri topla
- Log'ları yapılandırılmış formata dönüştür
- Nedensel alt ağaç oluştur
- LLM ile kök neden analizi yap
- Teşhis hipotezi oluştur

**Yapılandırma Parametreleri:**
- log_lookback_minutes: Kaç dakikalık log alınacak (örn: 5)
- max_log_lines: Maksimum log satırı (örn: 1000)
- llm_timeout_seconds: LLM yanıt zaman aşımı (örn: 30)
- confidence_threshold: Minimum güven skoru (örn: 0.7)

**Algoritma:**

```
HataTespit olayı geldiğinde:
    her hatalı_pod için:
        loglar = loki.sorgula(
            pod=hatalı_pod,
            başlangıç=şimdi - log_lookback_minutes,
            bitiş=şimdi,
            limit=max_log_lines
        )
        
        metrikler = prometheus.sorgula(
            pod=hatalı_pod,
            aynı_zaman_aralığı
        )
        
        kubernetes_olayları = k8s.olayları_getir(hatalı_pod)
        
        yapılandırılmış_loglar = dönüştür(loglar)
        nedensel_ağaç = nedensel_ağaç_oluştur(yapılandırılmış_loglar)
        
        prompt = teşhis_prompt_şablonu.doldur(
            loglar=yapılandırılmış_loglar,
            metrikler=metrikler,
            olaylar=kubernetes_olayları
        )
        
        llm_yanıtı = llm.sor(prompt)
        hipotez = parse_et(llm_yanıtı)
        
        eğer hipotez.güven >= confidence_threshold:
            yayınla(TeşhisTamamlandı, hipotez, nedensel_ağaç)
        değilse:
            daha_fazla_bilgi_topla_ve_tekrar_dene()
```

**Log Dönüşüm Kuralları:**
- Zaman damgası normalize edilir (UTC)
- Log seviyesi çıkarılır (ERROR, WARN, INFO, DEBUG)
- Kaynak belirlenir (uygulama, sistem, ağ)
- Stack trace'ler gruplandırılır
- Tekrarlayan log'lar sayılır ve özetlenir

**Nedensel Ağaç Oluşturma:**
- Her benzersiz hata mesajı bir düğüm olur
- Zamansal yakınlık: 1 saniye içindeki olaylar bağlanır
- Anahtar kelime eşleşmesi: Aynı kaynak/modül adı
- Bilinen pattern'ler: Veritabanı hatası → Sorgu hatası → API hatası

### 5.3 Üst-Bilişsel Ajan

**Sorumluluklar:**
- Teşhise göre çözüm stratejileri üret
- Mikro-ajanları oluştur ve yönet
- Paralel hipotez testi yap
- En iyi çözümü seç
- Kubernetes üzerinde eylemi uygula
- Sonucu doğrula

**Yapılandırma Parametreleri:**
- max_micro_agents: Maksimum mikro-ajan sayısı (örn: 5)
- max_reasoning_depth: Maksimum akıl yürütme derinliği (örn: 10)
- action_timeout_seconds: Eylem zaman aşımı (örn: 60)
- verification_wait_seconds: Doğrulama bekleme süresi (örn: 30)

**Olası Çözüm Stratejileri:**

Pod Yeniden Başlatma:
- Uygun durumlar: Bellek sızıntısı, kilitlenme, geçici hata
- Eylem: kubectl delete pod
- Risk: Düşük
- Süre: 10-30 saniye

Yatay Ölçekleme:
- Uygun durumlar: Yük artışı, kapasite yetersizliği
- Eylem: kubectl scale deployment --replicas=N
- Risk: Düşük-Orta
- Süre: 30-60 saniye

Dikey Ölçekleme:
- Uygun durumlar: Kaynak limitleri yetersiz
- Eylem: kubectl patch deployment (resources.limits)
- Risk: Orta
- Süre: 60-120 saniye

Yapılandırma Değişikliği:
- Uygun durumlar: Yanlış yapılandırma, bağlantı havuzu boyutu
- Eylem: kubectl patch configmap
- Risk: Orta-Yüksek
- Süre: 30-60 saniye

Bağımlılık Yeniden Başlatma:
- Uygun durumlar: Bağımlı servis sorunu
- Eylem: İlgili servisi yeniden başlat
- Risk: Yüksek
- Süre: 60-120 saniye

**Mikro-Ajan Yapısı:**

Her mikro-ajan şunları içerir:
- Hipotez: Test edilecek çözüm
- Kanıt listesi: Hipotezi destekleyen/çürüten veriler
- Güven skoru: 0-1 arası
- Derinlik: Kaç iterasyon yapıldı
- Durum: Çalışıyor/Tamamlandı/İptal

Mikro-ajan döngüsü:
```
başlat(hipotez):
    güven = başlangıç_güven_hesapla(hipotez, teşhis)
    derinlik = 0
    
    while güven < eşik ve derinlik < max_derinlik:
        soru = oluştur_soru(hipotez, mevcut_kanıtlar)
        llm_yanıtı = llm.sor(soru)
        yeni_kanıt = parse_et(llm_yanıtı)
        kanıtlar.ekle(yeni_kanıt)
        güven = güncelle_güven(kanıtlar)
        derinlik += 1
    
    döndür(hipotez, güven, kanıtlar)
```

**Karar Algoritması:**

```
TeşhisTamamlandı olayı geldiğinde:
    olası_çözümler = üret_çözümler(teşhis)
    
    mikro_ajanlar = []
    her çözüm için:
        ajan = MikroAjan(çözüm)
        mikro_ajanlar.ekle(ajan)
    
    # Paralel çalıştır
    sonuçlar = paralel_çalıştır(mikro_ajanlar)
    
    # En yüksek güvenli çözümü seç
    en_iyi = max(sonuçlar, key=güven_skoru)
    
    eğer en_iyi.güven >= karar_eşiği:
        uygula(en_iyi.çözüm)
        bekle(verification_wait_seconds)
        başarılı = doğrula()
        
        eğer başarılı:
            yayınla(İyileşmeTamamlandı, başarılı=true)
        değilse:
            geri_al()
            yayınla(İyileşmeTamamlandı, başarılı=false)
    değilse:
        yayınla(İyileşmeTamamlandı, başarılı=false, neden="düşük_güven")
```

### 5.4 Bilgi Ajanı

**Sorumluluklar:**
- İyileşme olaylarını kaydet
- Vektör embedding'leri oluştur
- Benzer geçmiş olayları bul
- Konu bölümleme yap
- Proaktif tahminler üret
- Eski bilgileri temizle

**Yapılandırma Parametreleri:**
- embedding_model: Kullanılacak embedding modeli
- similarity_threshold: Benzerlik eşiği (örn: 0.8)
- max_local_events: Yerel BN'de tutulacak maksimum olay (örn: 100)
- knowledge_ttl_days: Bilgi geçerlilik süresi (örn: 90)

**Veri Yapıları:**

Olay Kaydı:
```
{
    "id": "uuid",
    "timestamp": "ISO8601",
    "namespace": "string",
    "pod_name": "string",
    "error_type": "string",
    "diagnosis": {
        "hypothesis": "string",
        "confidence": "float",
        "root_cause": "string",
        "evidence": ["string"]
    },
    "solution": {
        "strategy": "string",
        "actions": ["string"],
        "duration_ms": "int"
    },
    "outcome": {
        "success": "bool",
        "verification_method": "string",
        "notes": "string"
    },
    "embedding": [float * 1536]
}
```

**Konu Bölümleme Algoritması:**

```
yeni_olay geldiğinde:
    embedding = oluştur_embedding(olay)
    
    # Mevcut konulara benzerlik kontrolü
    konular = qdrant.tüm_konuları_getir()
    
    her konu için:
        benzerlik = kosinüs_benzerliği(embedding, konu.merkez)
        eğer benzerlik > eşik:
            konu.olaylar.ekle(olay)
            konu.merkez = güncelle_merkez(konu.olaylar)
            döndür
    
    # Yeni konu oluştur
    yeni_konu = Konu(
        id=uuid(),
        ad=otomatik_ad_üret(olay),
        merkez=embedding,
        olaylar=[olay]
    )
    konular.ekle(yeni_konu)
```

**Proaktif Tahmin:**

```
periyodik olarak (her saat):
    son_metrikler = prometheus.sorgula(son_1_saat)
    
    her namespace için:
        trend = hesapla_trend(son_metrikler)
        
        eğer trend.cpu_artış > eşik:
            benzer_olaylar = qdrant.benzer_bul(
                "yüksek cpu kullanımı",
                konu="kaynak_sorunları"
            )
            
            eğer benzer_olaylar var ve çoğu hata ile sonuçlanmış:
                yayınla(ProaktifUyarı, 
                    mesaj="CPU artışı tespit edildi, olası hata riski",
                    önerilen_eylem=benzer_olaylar[0].çözüm
                )
```

---

## Bölüm 6: Özel Kaynak Tanımları (CRD)

### 6.1 SelfHealingPolicy CRD

Bu CRD, kendi kendini iyileştirme politikasını tanımlar. Hangi kaynakların izleneceğini, eşik değerlerini ve izin verilen eylemleri belirtir.

**Spec Alanları:**

targetNamespaces:
- İzlenecek namespace'lerin listesi
- Boş liste tüm namespace'ler anlamına gelir
- Tür: string dizisi

targetLabels:
- İzlenecek pod'ları filtrelemek için label selector
- Örnek: app=myapp, tier=backend
- Tür: key-value çiftleri

thresholds:
- cpu: CPU kullanım eşiği (0-1 arası)
- memory: Bellek kullanım eşiği (0-1 arası)
- latencyMs: Gecikme eşiği (milisaniye)
- errorRate: Hata oranı eşiği (0-1 arası)
- Tür: nesne

allowedActions:
- İzin verilen eylem listesi
- Değerler: restart, scale, updateConfig, updateResources
- Tür: string dizisi

llmConfig:
- provider: LLM sağlayıcısı (claude, openai, gemini, ollama)
- model: Model adı
- apiKeySecret: API anahtarını içeren Secret adı
- timeout: Zaman aşımı (saniye)
- Tür: nesne

notifications:
- enabled: Bildirim açık mı
- slackWebhook: Slack webhook URL'i (opsiyonel)
- email: E-posta adresi (opsiyonel)
- Tür: nesne

**Status Alanları:**

observedGeneration:
- Son işlenen spec generation'ı
- Tür: integer

activeHealings:
- Devam eden iyileştirme sayısı
- Tür: integer

lastHealingTime:
- Son iyileştirme zamanı
- Tür: timestamp

conditions:
- Standart Kubernetes condition'ları
- Tür: condition dizisi

### 6.2 HealingEvent CRD

Bu CRD, bir iyileştirme olayının kaydını tutar. Her iyileştirme girişimi için bir HealingEvent oluşturulur.

**Spec Alanları:**

policyRef:
- İlgili SelfHealingPolicy'nin adı
- Tür: string

targetPod:
- Hedef pod adı
- Tür: string

targetNamespace:
- Hedef namespace
- Tür: string

triggerReason:
- Tetiklenme nedeni
- Değerler: highCpu, highMemory, highLatency, highErrorRate, crashLoop
- Tür: string

**Status Alanları:**

phase:
- Mevcut aşama
- Değerler: Containing, Diagnosing, Healing, Verifying, Completed, Failed
- Tür: string

startTime:
- Başlangıç zamanı
- Tür: timestamp

endTime:
- Bitiş zamanı (tamamlandıysa)
- Tür: timestamp

diagnosis:
- hypothesis: Teşhis hipotezi
- confidence: Güven skoru
- rootCause: Kök neden
- Tür: nesne

appliedActions:
- Uygulanan eylemler listesi
- Her eylem: action, timestamp, result
- Tür: nesne dizisi

outcome:
- success: Başarılı mı
- message: Sonuç mesajı
- Tür: nesne

---

## Bölüm 7: Test Stratejisi

### 7.1 Birim Testleri

**Sınırlama Ajanı Testleri:**
- Eşik hesaplama doğruluğu
- NetworkPolicy oluşturma doğruluğu
- Komşu müzakere mantığı
- Hata kümesi yönetimi

**Teşhis Ajanı Testleri:**
- Log parse etme doğruluğu
- Nedensel ağaç oluşturma
- LLM prompt oluşturma
- Yanıt parse etme

**Üst-Bilişsel Ajan Testleri:**
- Strateji üretme mantığı
- Mikro-ajan yaşam döngüsü
- Karar algoritması
- Eylem uygulama

**Bilgi Ajanı Testleri:**
- Embedding oluşturma
- Benzerlik arama
- Konu bölümleme
- TTL yönetimi

### 7.2 Entegrasyon Testleri

**Gözlemlenebilirlik Entegrasyonu:**
- Prometheus sorgu testi
- Loki sorgu testi
- Kubernetes API testi

**LLM Entegrasyonu:**
- API bağlantı testi
- Timeout yönetimi
- Hata durumları

**Veritabanı Entegrasyonu:**
- Qdrant bağlantı testi
- Redis bağlantı testi
- Veri tutarlılığı

### 7.3 Uçtan Uca Testler

**Senaryo 1: Yüksek CPU**
- Bir pod'a CPU yükü ver (stress-ng)
- Sınırlama ajanının tespit etmesini bekle
- Teşhisin "yüksek cpu yükü" olmasını doğrula
- Uygulanan eylemin mantıklı olduğunu kontrol et
- Sistem normale dönene kadar bekle

**Senaryo 2: Bellek Sızıntısı**
- Bellek sızdıran bir uygulama çalıştır
- OOM killer tetiklenmeden önce tespit edilmesini bekle
- Teşhisin bellek sızıntısı olduğunu doğrula
- Pod restart'ın uygulandığını kontrol et

**Senaryo 3: Veritabanı Bağlantı Sorunu**
- Veritabanı bağlantısını kes
- Uygulama hatalarının tespit edilmesini bekle
- Teşhisin bağlantı sorunu olduğunu doğrula
- Uygun eylemin uygulandığını kontrol et

**Senaryo 4: Cascading Failure**
- Bir servisi kapat
- Bağımlı servislerin etkilenmesini gözlemle
- Sınırlamanın yayılmayı önlediğini doğrula
- Tüm sistemin kurtarıldığını kontrol et

### 7.4 Kaos Mühendisliği

**Chaos Mesh Kullanımı:**
- Pod kill: Rastgele pod öldürme
- Network delay: Ağ gecikmesi ekleme
- Network partition: Ağ bölünmesi
- CPU stress: CPU yükü
- Memory stress: Bellek yükü
- Disk fill: Disk doldurma

**Test Senaryoları:**
- Tek pod hatası
- Çoklu pod hatası
- Node hatası
- Ağ bölünmesi
- Bağımlılık hatası

---

## Bölüm 8: Performans Hedefleri

### 8.1 Zaman Hedefleri

| Aşama | Hedef Süre | Maksimum |
|-------|-----------|----------|
| Hata Tespiti | 5 saniye | 10 saniye |
| İzolasyon | 3 saniye | 5 saniye |
| Log Toplama | 5 saniye | 10 saniye |
| LLM Teşhis | 10 saniye | 20 saniye |
| Strateji Belirleme | 5 saniye | 10 saniye |
| Eylem Uygulama | 10 saniye | 30 saniye |
| Doğrulama | 30 saniye | 60 saniye |
| **Toplam** | **68 saniye** | **145 saniye** |

Hedef: Ortalama iyileşme süresi 60 saniyenin altında.

### 8.2 Kaynak Hedefleri

| Metrik | Hedef | Maksimum |
|--------|-------|----------|
| CPU Kullanımı | %10 | %15 |
| Bellek Kullanımı | 256 MB | 512 MB |
| API Çağrısı/dakika | 100 | 200 |
| LLM Token/iyileşme | 2000 | 5000 |

### 8.3 Güvenilirlik Hedefleri

| Metrik | Hedef |
|--------|-------|
| Doğru Tespit Oranı | %95+ |
| Yanlış Pozitif Oranı | %5- |
| Başarılı İyileşme Oranı | %90+ |
| Otomatik Çözüm Oranı | %80+ |

---

## Bölüm 9: Güvenlik Hususları

### 9.1 RBAC Yapılandırması

Operatör için gerekli minimum yetkiler:

Pods:
- get, list, watch: Tüm namespace'lerde
- delete: İzin verilen namespace'lerde (restart için)

Deployments:
- get, list, watch: Tüm namespace'lerde
- patch, update: İzin verilen namespace'lerde (scale için)

ConfigMaps:
- get, list: Tüm namespace'lerde
- patch, update: İzin verilen namespace'lerde

Secrets:
- get: Sadece kendi namespace'inde (API anahtarları için)

NetworkPolicies:
- create, delete: İzin verilen namespace'lerde

Events:
- create: Tüm namespace'lerde (audit için)

Custom Resources:
- Tam yetki: SelfHealingPolicy, HealingEvent

### 9.2 API Anahtarı Yönetimi

- LLM API anahtarları Kubernetes Secret'ta saklanır
- Secret adı SelfHealingPolicy'de belirtilir
- Operatör Secret'ı okur, bellekte tutar
- Log'lara API anahtarı yazılmaz

### 9.3 Ağ Güvenliği

- Operatör sadece gerekli servislere bağlanır
- Prometheus, Loki: Küme içi
- LLM API: Küme dışı (HTTPS)
- Qdrant, Redis: Küme içi

- Egress NetworkPolicy ile sınırlandırılabilir

### 9.4 Audit Log

- Tüm iyileştirme eylemleri loglanır
- Kim tetikledi (otomatik/manuel)
- Ne zaman
- Hangi eylem uygulandı
- Sonuç ne oldu

---

## Bölüm 10: Dağıtım ve Operasyon

### 10.1 Helm Chart Yapısı

```
recist/
├── Chart.yaml
├── values.yaml
├── templates/
│   ├── deployment.yaml
│   ├── serviceaccount.yaml
│   ├── clusterrole.yaml
│   ├── clusterrolebinding.yaml
│   ├── crd-selfhealingpolicy.yaml
│   ├── crd-healingevent.yaml
│   ├── configmap.yaml
│   └── secret.yaml
```

### 10.2 Değerler (values.yaml)

```yaml
replicaCount: 1

image:
  repository: recist
  tag: latest
  pullPolicy: IfNotPresent

resources:
  limits:
    cpu: 500m
    memory: 512Mi
  requests:
    cpu: 100m
    memory: 256Mi

prometheus:
  url: http://prometheus:9090

loki:
  url: http://loki:3100

qdrant:
  url: http://qdrant:6334

redis:
  url: redis://redis:6379

llm:
  provider: claude
  model: claude-3-sonnet
  # apiKey: Secret'tan okunur
```

### 10.3 Kurulum Adımları

1. Ön gereksinimler kurulur:
   - Prometheus + Loki (gözlemlenebilirlik)
   - Qdrant (vektör veritabanı)
   - Redis (önbellek)

2. CRD'ler uygulanır:
   - kubectl apply -f crds/

3. Helm chart kurulur:
   - helm install recist ./helm/recist

4. LLM API anahtarı eklenir:
   - kubectl create secret generic llm-api-key --from-literal=key=xxx

5. SelfHealingPolicy oluşturulur:
   - kubectl apply -f examples/policy.yaml

### 10.4 İzleme ve Uyarılar

Operatör kendi metriklerini de yayınlar:

- recist_healings_total: Toplam iyileştirme sayısı
- recist_healings_success_total: Başarılı iyileştirme sayısı
- recist_healing_duration_seconds: İyileştirme süresi histogramı
- recist_llm_requests_total: LLM istek sayısı
- recist_llm_latency_seconds: LLM gecikme histogramı

Önerilen Uyarılar:
- İyileştirme başarı oranı %80'in altına düşerse
- İyileştirme süresi 2 dakikayı aşarsa
- LLM hata oranı %10'u aşarsa

---

## Bölüm 11: Proje Yol Haritası

### Aşama 1: Temel Altyapı (Hafta 1-3)

- Rust projesi oluşturma
- kube-rs entegrasyonu
- Temel operatör iskeleti
- CRD tanımları
- Yapılandırma yönetimi
- Loglama altyapısı

### Aşama 2: Sınırlama Ajanı (Hafta 4-5)

- Prometheus entegrasyonu
- Eşik kontrolü
- Hata tespiti
- NetworkPolicy oluşturma
- İzolasyon mekanizması
- Birim testleri

### Aşama 3: Teşhis Ajanı (Hafta 6-8)

- Loki entegrasyonu
- Log toplama ve dönüştürme
- LLM entegrasyonu
- Prompt şablonları
- Nedensel ağaç oluşturma
- Birim testleri

### Aşama 4: Üst-Bilişsel Ajan (Hafta 9-11)

- Strateji üretimi
- Mikro-ajan mimarisi
- Paralel çalıştırma
- Karar mekanizması
- Eylem uygulama
- Birim testleri

### Aşama 5: Bilgi Ajanı (Hafta 12-13)

- Qdrant entegrasyonu
- Embedding oluşturma
- Benzerlik araması
- Konu bölümleme
- Redis önbellek
- Birim testleri

### Aşama 6: Entegrasyon ve Test (Hafta 14-15)

- Uçtan uca testler
- Kaos mühendisliği testleri
- Performans testleri
- Hata ayıklama
- Dokümantasyon

### Aşama 7: Dağıtım (Hafta 16)

- Helm chart
- CI/CD pipeline
- Container image
- README ve örnekler
- Demo video

---

## Bölüm 12: Referanslar

### Akademik Paper

- Başlık: Bio-inspired Agentic Self-healing Framework for Resilient Distributed Computing Continuum Systems
- Yazarlar: Alaa Saleh, Praveen Kumar Donta, Roberto Morabito, Sasu Tarkoma, Schahram Dustdar, Susanna Pirttikangas, Lauri Lovén
- arXiv: 2601.00339
- Tarih: 1 Ocak 2026

### Teknoloji Dokümantasyonu

- Kubernetes: kubernetes.io/docs
- kube-rs: kube.rs
- Prometheus: prometheus.io/docs
- Loki: grafana.com/docs/loki
- Qdrant: qdrant.tech/documentation
- Anthropic Claude: docs.anthropic.com

---

## Son Notlar

Bu doküman, ReCiSt projesini sıfırdan geliştirmek için gereken tüm bağlamı içerir. Kod içermez çünkü amaç, bir yapay zeka veya geliştiricinin bu dokümanı okuyarak projeyi tam olarak anlaması ve kodlamasıdır.

Doküman şunları sağlar:
- Problemin net tanımı
- Biyolojik benzetmenin detaylı açıklaması
- Her katmanın sorumluluklarının tam listesi
- Teknoloji seçimlerinin gerekçeleri
- Veri akışının tam haritası
- Algoritmaların sözde kod ile açıklaması
- Test stratejisinin detayları
- Performans hedeflerinin tanımı
- Güvenlik hususlarının listesi
- Dağıtım adımlarının açıklaması

Bu dokümanla birlikte, bir geliştirici veya yapay zeka asistanı projeyi baştan sona kodlayabilir.

---

Doküman Sonu
