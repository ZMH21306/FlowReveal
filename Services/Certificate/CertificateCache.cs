using System;
using System.Collections.Concurrent;
using System.Security.Cryptography.X509Certificates;

namespace FlowReveal.Services.Certificate;

public class CertificateCache
{
    private readonly ConcurrentDictionary<string, X509Certificate2> _cache = new();
    private readonly CertificateGenerator _generator;
    private X509Certificate2? _rootCa;
    private readonly object _rootCaLock = new();

    public CertificateCache(CertificateGenerator generator)
    {
        _generator = generator;
    }

    public X509Certificate2 GetRootCertificate()
    {
        if (_rootCa == null)
        {
            lock (_rootCaLock)
            {
                if (_rootCa == null)
                {
                    _rootCa = _generator.GenerateRootCertificate();
                }
            }
        }
        
        return _rootCa;
    }

    public X509Certificate2 GetCertificateForHost(string hostname)
    {
        if (string.IsNullOrEmpty(hostname))
            throw new ArgumentNullException(nameof(hostname));

        string key = hostname.ToLowerInvariant();

        return _cache.GetOrAdd(key, _ => 
        {
            var rootCa = GetRootCertificate();
            return _generator.GenerateLeafCertificate(rootCa, hostname);
        });
    }

    public void Clear()
    {
        foreach (var cert in _cache.Values)
        {
            cert.Dispose();
        }
        _cache.Clear();
    }

    public void Remove(string hostname)
    {
        if (_cache.TryRemove(hostname.ToLowerInvariant(), out var cert))
        {
            cert.Dispose();
        }
    }

    public int Count => _cache.Count;
}