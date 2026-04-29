using System;
using System.Collections.Generic;
using System.Security.Cryptography;
using System.Security.Cryptography.X509Certificates;

namespace FlowReveal.Services.Certificate;

public class CertificateGenerator
{
    private const int KeySize = 2048;
    private readonly RandomNumberGenerator _rng = RandomNumberGenerator.Create();

    public X509Certificate2 GenerateRootCertificate(string subjectName = "CN=FlowReveal Root CA")
    {
        using var rsa = RSA.Create(KeySize);
        
        var request = new CertificateRequest(
            subjectName,
            rsa,
            HashAlgorithmName.SHA256,
            RSASignaturePadding.Pkcs1
        );

        request.CertificateExtensions.Add(
            new X509BasicConstraintsExtension(true, true, 0, true)
        );

        request.CertificateExtensions.Add(
            new X509KeyUsageExtension(
                X509KeyUsageFlags.KeyCertSign | X509KeyUsageFlags.CrlSign, 
                true
            )
        );

        request.CertificateExtensions.Add(
            new X509EnhancedKeyUsageExtension(
                new OidCollection
                {
                    new Oid("1.3.6.1.5.5.7.3.1"),
                    new Oid("1.3.6.1.5.5.7.3.2")
                },
                true
            )
        );

        var serialNumber = GenerateSerialNumber();

        var cert = request.CreateSelfSigned(
            DateTimeOffset.Now.AddDays(-1),
            DateTimeOffset.Now.AddYears(10)
        );

        cert.FriendlyName = "FlowReveal Root CA";

        return new X509Certificate2(cert.Export(X509ContentType.Pfx), string.Empty);
    }

    public X509Certificate2 GenerateLeafCertificate(X509Certificate2 rootCa, string hostname)
    {
        using var rsa = RSA.Create(KeySize);

        var request = new CertificateRequest(
            $"CN={hostname}",
            rsa,
            HashAlgorithmName.SHA256,
            RSASignaturePadding.Pkcs1
        );

        request.CertificateExtensions.Add(
            new X509BasicConstraintsExtension(false, false, 0, false)
        );

        request.CertificateExtensions.Add(
            new X509KeyUsageExtension(
                X509KeyUsageFlags.DigitalSignature | X509KeyUsageFlags.KeyEncipherment, 
                true
            )
        );

        request.CertificateExtensions.Add(
            new X509EnhancedKeyUsageExtension(
                new OidCollection
                {
                    new Oid("1.3.6.1.5.5.7.3.1")
                },
                true
            )
        );

        var sanBuilder = new SubjectAlternativeNameBuilder();
        sanBuilder.AddDnsName(hostname);
        
        if (!hostname.StartsWith("*."))
        {
            sanBuilder.AddDnsName($"*.{hostname}");
        }

        request.CertificateExtensions.Add(sanBuilder.Build());

        var serialNumber = GenerateSerialNumber();

        using var rootPrivateKey = rootCa.GetRSAPrivateKey();
        
        var cert = request.Create(
            rootCa,
            DateTimeOffset.Now.AddMinutes(-5),
            DateTimeOffset.Now.AddDays(1),
            serialNumber
        );

        cert.FriendlyName = $"FlowReveal - {hostname}";

        var certWithKey = cert.CopyWithPrivateKey(rsa);
        
        return new X509Certificate2(certWithKey.Export(X509ContentType.Pfx), string.Empty);
    }

    private byte[] GenerateSerialNumber()
    {
        var serialNumber = new byte[20];
        _rng.GetBytes(serialNumber);
        serialNumber[0] &= 0x7F;
        return serialNumber;
    }
}