using System;
using System.Collections.Generic;
using System.Linq;

namespace FlowReveal.Services.Http;

public class TlsHandshakeMessage
{
    public byte[] RawData { get; set; } = Array.Empty<byte>();
    public TlsVersion Version { get; set; }
    public string Sni { get; set; } = string.Empty;
    public List<TlsCipherSuite> CipherSuites { get; set; } = new();
    public List<TlsExtension> Extensions { get; set; } = new();
}

public enum TlsVersion
{
    Unknown,
    Tls10,
    Tls11,
    Tls12,
    Tls13
}

public enum TlsCipherSuite : ushort
{
    Unknown = 0x0000,
    TLS_AES_128_GCM_SHA256 = 0x1301,
    TLS_AES_256_GCM_SHA384 = 0x1302,
    TLS_CHACHA20_POLY1305_SHA256 = 0x1303,
    TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256 = 0xC02B,
    TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256 = 0xC02F,
    TLS_ECDHE_ECDSA_WITH_AES_256_GCM_SHA384 = 0xC02C,
    TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384 = 0xC030,
    TLS_ECDHE_ECDSA_WITH_CHACHA20_POLY1305_SHA256 = 0xCCA9,
    TLS_ECDHE_RSA_WITH_CHACHA20_POLY1305_SHA256 = 0xCCA8,
    TLS_RSA_WITH_AES_128_GCM_SHA256 = 0x009C,
    TLS_RSA_WITH_AES_256_GCM_SHA384 = 0x009D
}

public class TlsExtension
{
    public ushort Type { get; set; }
    public byte[] Data { get; set; } = Array.Empty<byte>();
    public string? Name { get; set; }
}

public class TlsHandshakeParser
{
    private const byte HandshakeTypeClientHello = 0x01;
    
    private const ushort ExtensionServerName = 0x0000;
    private const ushort ExtensionSupportedVersions = 0x002B;
    private const ushort ExtensionSignatureAlgorithms = 0x000D;

    public bool TryParseClientHello(byte[] data, out TlsHandshakeMessage result)
    {
        result = new TlsHandshakeMessage();
        
        if (data.Length < 42)
            return false;

        int offset = 0;
        
        if (data[offset] != HandshakeTypeClientHello)
            return false;
        offset++;

        int handshakeLength = ReadInt24(data, offset);
        offset += 3;

        if (offset + handshakeLength > data.Length)
            return false;

        ushort legacyVersion = ReadUInt16(data, offset);
        offset += 2;

        TlsVersion version = legacyVersion switch
        {
            0x0301 => TlsVersion.Tls10,
            0x0302 => TlsVersion.Tls11,
            0x0303 => TlsVersion.Tls12,
            _ => TlsVersion.Unknown
        };

        byte[] random = new byte[32];
        Array.Copy(data, offset, random, 0, 32);
        offset += 32;

        byte sessionIdLength = data[offset];
        offset++;
        
        if (sessionIdLength > 32 || offset + sessionIdLength > data.Length)
            return false;
        
        offset += sessionIdLength;

        ushort cipherSuitesLength = ReadUInt16(data, offset);
        offset += 2;
        
        if (cipherSuitesLength % 2 != 0 || offset + cipherSuitesLength > data.Length)
            return false;

        List<TlsCipherSuite> cipherSuites = new();
        for (int i = 0; i < cipherSuitesLength; i += 2)
        {
            ushort suite = ReadUInt16(data, offset + i);
            cipherSuites.Add((TlsCipherSuite)suite);
        }
        offset += cipherSuitesLength;

        byte compressionMethodsLength = data[offset];
        offset++;
        
        if (offset + compressionMethodsLength > data.Length)
            return false;
        
        offset += compressionMethodsLength;

        List<TlsExtension> extensions = new();
        ushort extensionsLength = ReadUInt16(data, offset);
        offset += 2;
        
        if (offset + extensionsLength > data.Length)
            return false;

        int extensionsEnd = offset + extensionsLength;
        
        while (offset < extensionsEnd)
        {
            ushort extensionType = ReadUInt16(data, offset);
            offset += 2;
            
            ushort extensionLength = ReadUInt16(data, offset);
            offset += 2;
            
            if (offset + extensionLength > extensionsEnd)
                break;

            byte[] extensionData = new byte[extensionLength];
            Array.Copy(data, offset, extensionData, 0, extensionLength);
            offset += extensionLength;

            string? extensionName = extensionType switch
            {
                ExtensionServerName => "ServerName",
                ExtensionSupportedVersions => "SupportedVersions",
                ExtensionSignatureAlgorithms => "SignatureAlgorithms",
                _ => null
            };

            extensions.Add(new TlsExtension
            {
                Type = extensionType,
                Data = extensionData,
                Name = extensionName
            });
        }

        string sni = ExtractSni(extensions);
        
        TlsVersion tls13Version = ExtractTls13Version(extensions);
        if (tls13Version != TlsVersion.Unknown)
            version = tls13Version;

        result = new TlsHandshakeMessage
        {
            RawData = data,
            Version = version,
            Sni = sni,
            CipherSuites = cipherSuites,
            Extensions = extensions
        };

        return true;
    }

    private string ExtractSni(List<TlsExtension> extensions)
    {
        var sniExtension = extensions.FirstOrDefault(e => e.Type == ExtensionServerName);
        
        if (sniExtension == null || sniExtension.Data.Length < 3)
            return string.Empty;

        try
        {
            int offset = 0;
            ushort listLength = ReadUInt16(sniExtension.Data, offset);
            offset += 2;

            if (offset + listLength > sniExtension.Data.Length)
                return string.Empty;

            while (offset < sniExtension.Data.Length)
            {
                byte nameType = sniExtension.Data[offset];
                offset++;

                ushort nameLength = ReadUInt16(sniExtension.Data, offset);
                offset += 2;

                if (offset + nameLength > sniExtension.Data.Length)
                    break;

                if (nameType == 0x00)
                {
                    return System.Text.Encoding.ASCII.GetString(sniExtension.Data, offset, nameLength);
                }

                offset += nameLength;
            }
        }
        catch
        {
        }

        return string.Empty;
    }

    private TlsVersion ExtractTls13Version(List<TlsExtension> extensions)
    {
        var versionExtension = extensions.FirstOrDefault(e => e.Type == ExtensionSupportedVersions);
        
        if (versionExtension == null || versionExtension.Data.Length < 3)
            return TlsVersion.Unknown;

        try
        {
            int offset = 0;
            ushort versionsLength = ReadUInt16(versionExtension.Data, offset);
            offset += 2;

            for (int i = 0; i < versionsLength; i += 2)
            {
                ushort version = ReadUInt16(versionExtension.Data, offset + i);
                if (version == 0x0304)
                    return TlsVersion.Tls13;
            }
        }
        catch
        {
        }

        return TlsVersion.Unknown;
    }

    private ushort ReadUInt16(byte[] data, int offset)
    {
        return (ushort)((data[offset] << 8) | data[offset + 1]);
    }

    private int ReadInt24(byte[] data, int offset)
    {
        return (data[offset] << 16) | (data[offset + 1] << 8) | data[offset + 2];
    }
}