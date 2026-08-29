import logoUrl from '../assets/mydns.svg';

export function Logo() {
  return (
    <div className="brand">
      <img src={logoUrl} alt="MyDNS compass" width={32} height={32} />
      <span>MyDNS</span>
    </div>
  );
}
